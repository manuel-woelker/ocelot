use crate::compile_program::compile_program;
use crate::core_module::default_core_module;
use crate::engine::Engine;
use crate::native_scenario_step::NativeScenarioStep;
use crate::native_translation_unit::NativeTranslationUnit;
use crate::prepare_eval_input::prepare_eval_input;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_base::file_path::FilePath;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::{OcelotResult, OptionExt};
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_base::source_file::SourceFile;
use ocelot_codegen_c::c_emitter::{CEmitter, runtime_header};
use ocelot_pal::pal::PalHandle;
use ocelot_semantic::source_file_kind::SourceFileKind;

fn roots_in(statements: &[ocelot_ast::statement::Statement]) -> Vec<SharedString> {
    statements
        .iter()
        .filter_map(|s| {
            if let StatementKind::Let(v) = &s.kind {
                Some(v.identifier.name.clone())
            } else {
                None
            }
        })
        .collect()
}

fn main_start(roots: usize) -> String {
    format!(
        "int main(int argc, char **argv) {{ oc_init_v1(argc == 3 && !strcmp(argv[1], \"--ocelot-report\") ? argv[2] : NULL); OcValue roots[{}] = {{{{0,0}}}}; (void)roots;\n",
        roots.max(1)
    )
}

fn main_end(roots: usize) -> String {
    let cleanup = if roots == 0 {
        String::new()
    } else {
        format!("for (size_t i=0; i<{roots}; ++i) oc_release(roots[i]);")
    };
    format!("{cleanup} oc_finish(); return 0; }}\n")
}

/// Generates a file executable after using the normal engine frontend.
pub fn file_source(pal: PalHandle, path: &FilePath) -> OcelotResult<NativeTranslationUnit> {
    let program = Engine::new(pal).prepare_file(path.clone())?;
    let module = program
        .modules
        .iter()
        .find(|m| m.source_file.path == program.entry_path)
        .context("missing entry module")?;
    let statements = module
        .compilation_unit
        .statements()
        .cloned()
        .collect::<Vec<_>>();
    let roots = roots_in(&statements);
    let mut code = format!("{}\n#include <string.h>\n", runtime_header());
    code.push_str(&CEmitter::new(&program, "file").emit_step(
        &statements,
        &module.source_file,
        &[],
        &roots,
        None,
    )?);
    code.push_str(&main_start(roots.len()));
    if module.kind == SourceFileKind::Module {
        let name = program
            .symbol_table
            .qualify_function_name(&module.module_name, "main");
        let index = program
            .symbol_table
            .resolve_function_exact(&name)
            .context(format!(
                "module `{}` does not define a `main()` entrypoint",
                module.module_name
            ))?;
        code.push_str(&format!(
            "OcValue result = oc_file_f{}(); oc_release(result);\n",
            index.as_usize()
        ));
    } else {
        code.push_str("OcValue result = oc_file_step(roots); oc_release(result);\n");
    }
    code.push_str(&main_end(roots.len()));
    Ok(NativeTranslationUnit {
        source: code.into(),
        deferred_error: None,
        step_count: 1,
    })
}

/// Compiles a finite embedded scenario without invoking the interpreter.
pub fn scenario_source(
    pal: PalHandle,
    base: &FilePath,
    steps: &[NativeScenarioStep],
) -> OcelotResult<NativeTranslationUnit> {
    let entry = base.join("__embedded_main.ocelot-script");
    let mut retained = String::new();
    let mut roots = Vec::new();
    let mut definitions = format!("{}\n#include <string.h>\n", runtime_header());
    let mut driver = String::new();
    let mut deferred_error = None;
    let mut count = 0;
    for (step_index, step) in steps.iter().enumerate() {
        let (source, eval) = match step {
            NativeScenarioStep::Execute(s) => (s.to_string(), false),
            NativeScenarioStep::Eval(s) => (format!("{s};"), true),
        };
        let mut diagnostics = SourceDiagnostics::default();
        if eval
            && let Err(error) = prepare_eval_input(
                &SourceFile::new(entry.clone(), source.as_str()),
                &mut diagnostics,
            )
        {
            deferred_error = Some(error);
            break;
        }
        let offset = if retained.is_empty() {
            0
        } else {
            retained.len() + 1
        };
        let candidate = if retained.is_empty() {
            source
        } else {
            format!("{retained}\n{source}")
        };
        let file = SourceFile::new(entry.clone(), candidate.as_str());
        let program = match compile_program(
            &*pal,
            base,
            &entry,
            &[default_core_module()],
            &[file],
            &mut diagnostics,
        ) {
            Ok(program) => program,
            Err(error) => {
                deferred_error = Some(if diagnostics.has_errors() {
                    error.with_source(ocelot_base::error::OcelotError::message(
                        render_source_diagnostics(&diagnostics.diagnostics),
                    ))
                } else {
                    error
                });
                break;
            }
        };
        let module = program
            .modules
            .iter()
            .find(|m| m.source_file.path == entry)
            .context("missing embedded entry")?;
        let statements = module
            .compilation_unit
            .statements()
            .filter(|s| s.span.start() >= offset)
            .cloned()
            .collect::<Vec<_>>();
        let initial_roots = roots.clone();
        let expression = if eval {
            match &statements.first().context("missing eval statement")?.kind {
                StatementKind::Expression(e) => Some(&e.expression),
                _ => ocelot_base::bail!("invalid prepared eval"),
            }
        } else {
            roots.extend(roots_in(&statements));
            None
        };
        definitions.push_str(
            &CEmitter::new(&program, format!("s{step_index}")).emit_step(
                if eval { &[] } else { &statements },
                &module.source_file,
                &initial_roots,
                &roots,
                expression,
            )?,
        );
        driver.push_str(&format!(
            "OcValue step{step_index} = oc_s{step_index}_step(roots);\n"
        ));
        if eval {
            driver.push_str(&format!("oc_eval(step{step_index});\n"));
        } else {
            retained = candidate;
        }
        driver.push_str(&format!("oc_release(step{step_index});\n"));
        count += 1;
    }
    definitions.push_str(&main_start(roots.len()));
    definitions.push_str(&driver);
    definitions.push_str(&main_end(roots.len()));
    Ok(NativeTranslationUnit {
        source: definitions.into(),
        deferred_error,
        step_count: count,
    })
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
mod tests {
    use super::{file_source, scenario_source};
    use crate::build_native::{build_native, runtime_archive};
    use crate::captured_process::CapturedProcess;
    use crate::native_scenario_step::NativeScenarioStep;
    use crate::native_scenario_step::NativeScenarioStep::{Eval, Execute};
    use crate::native_translation_unit::NativeTranslationUnit;
    use crate::tcc_toolchain::TccToolchain;
    use ocelot_base::file_path::FilePath;
    use ocelot_pal::pal::{Pal, PalHandle};
    use ocelot_pal::pal_real::PalReal;
    use ocelot_pal::process_command::ProcessCommand;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn workspace() -> (PalHandle, FilePath) {
        let pal = PalHandle::new(PalReal::new().unwrap());
        let root = FilePath::new(std::env::temp_dir().join(format!(
                "ocelot native tests {}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )));
        pal.create_directory_all(&root).unwrap();
        (pal, root)
    }

    fn run(
        pal: &dyn Pal,
        root: &FilePath,
        unit: &NativeTranslationUnit,
    ) -> (CapturedProcess, String) {
        let binary = root.join("program with spaces");
        let report = root.join("report");
        build_native(
            pal,
            &unit.source,
            &binary,
            &runtime_archive().unwrap(),
            &TccToolchain::repository(),
        )
        .unwrap();
        let output = CapturedProcess::run(
            pal,
            &ProcessCommand {
                executable: binary.into_shared_string(),
                arguments: vec!["--ocelot-report".into(), report.into_shared_string()],
                working_directory: Some(root.clone()),
                environment: Vec::new(),
            },
            Duration::from_secs(5),
        )
        .unwrap();
        let report = pal
            .read_file_to_string(&root.join("report"))
            .unwrap()
            .to_string();
        (output, report)
    }

    #[test]
    fn native_file_preserves_values_scope_returns_and_argument_order() {
        let (pal, root) = workspace();
        let path = root.join("main.ocelot-script");
        pal.write_file(
            &path,
            br#"
fun identity(name: string) -> string { name }
fun early() -> string {
    assert_eq(expected: { return "early"; }, actual: println("unreachable"));
    "wrong"
}
fun choose(flag: bool) -> string { if flag { identity(name: "yes") } else { "no" } }
let name = "outer";
{ let name = "${name} inner"; println(name); };
println(name);
println(choose(flag: true));
println(early());
println({ println("left"); "a" } != { println("right"); "b" });
test "not run" { assert(false); }
"#,
        )
        .unwrap();
        let unit = file_source(pal.clone(), &path).unwrap();
        // The artifact must retain all required source metadata itself.
        pal.remove_directory_all(&root).unwrap();
        pal.create_directory_all(&root).unwrap();
        let (output, report) = run(&*pal, &root, &unit);
        assert_eq!(output.exit_code, Some(0));
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "outer inner\nouter\nyes\nearly\nleft\nright\ntrue\n"
        );
        assert_eq!(report, "ocelot-ok-v1\n");
        pal.remove_directory_all(&root).unwrap();
    }

    #[test]
    fn native_scenario_retains_values_without_replaying_and_discards_eval_locals() {
        let (pal, root) = workspace();
        let steps = [
            Execute("let name = { println(\"once\"); \"Ada\" };".into()),
            Eval("{ let name = \"inner\"; name }".into()),
            Execute("let greeting = \"Hello ${name}!\";".into()),
            Eval("greeting".into()),
        ];
        let unit = scenario_source(pal.clone(), &root, &steps).unwrap();
        let (output, report) = run(&*pal, &root, &unit);
        assert_eq!(output.exit_code, Some(0));
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "once\n");
        assert_eq!(report, "ocelot-eval-v1\n\"Hello Ada!\"");
        pal.remove_directory_all(&root).unwrap();
    }

    #[test]
    fn native_prefix_runtime_failure_takes_precedence_over_later_frontend_error() {
        let (pal, root) = workspace();
        let steps: [NativeScenarioStep; 2] = [
            Execute("println(\"before\"); assert(false);".into()),
            Eval("missing".into()),
        ];
        let unit = scenario_source(pal.clone(), &root, &steps).unwrap();
        assert!(unit.deferred_error.is_some());
        let (output, report) = run(&*pal, &root, &unit);
        assert_eq!(output.exit_code, Some(101));
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "before\n");
        assert!(report.starts_with("ocelot-error-v1\nerror: assert condition was false"));
        assert!(!report.contains("missing"));
        pal.remove_directory_all(&root).unwrap();
    }

    #[test]
    fn native_codegen_is_deterministic_and_uses_exact_bytes() {
        let (pal, root) = workspace();
        let path = root.join("main.ocelot-script");
        pal.write_file(&path, "println(\"a\0é\");".as_bytes())
            .unwrap();
        let first = file_source(pal.clone(), &path).unwrap();
        let second = file_source(pal.clone(), &path).unwrap();
        assert_eq!(first.source, second.source);
        let (output, _) = run(&*pal, &root, &first);
        assert_eq!(output.stdout, "a\0é\n".as_bytes());
        pal.remove_directory_all(&root).unwrap();
    }

    #[test]
    fn a_failed_toolchain_preserves_the_previous_executable() {
        let (pal, root) = workspace();
        let binary = root.join("existing output");
        pal.write_file(&binary, b"previous artifact").unwrap();
        let mut toolchain = TccToolchain::repository();
        toolchain.compiler = "/nonexistent/ocelot-tcc".into();
        toolchain.compiler_arguments.clear();
        let error = build_native(
            &*pal,
            "int main(void) { return 0; }",
            &binary,
            &runtime_archive().unwrap(),
            &toolchain,
        )
        .unwrap_err();
        assert_eq!(
            pal.read_file_to_string(&binary).unwrap(),
            "previous artifact"
        );
        assert!(error.kind().to_string().contains("native build artifacts"));
        pal.remove_directory_all(&root).unwrap();
    }
}
