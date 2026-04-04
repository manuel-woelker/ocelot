use crate::discovered_test::DiscoveredTest;
use crate::failed_test_result::FailedTestResult;
use crate::loaded_module::LoadedModule;
use crate::loaded_program::LoadedProgram;
use crate::source_file_kind::SourceFileKind;
use crate::test_run_summary::TestRunSummary;
use ocelot_ast::item_kind::ItemKind;
use ocelot_base::assertion_error::render_assertion_error;
use ocelot_base::compilation_context::CompilationContext;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::error::ErrorKind;
use ocelot_base::error::OcelotError;
use ocelot_base::file_path::FilePath;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::OcelotResult;
use ocelot_base::result::OptionExt;
use ocelot_base::result::ResultExt;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_pal::pal::PalHandle;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::native_function::NativeFunctionRegistry;
use ocelot_semantic::native_function::default_native_function_registry;
use ocelot_semantic::program_environment::ProgramEnvironment;

const CORE_MODULE_NAME: &str = "core";
const CORE_MODULE_PATH: &str = "crates/engine/resources/core.ocelot";
const CORE_MODULE_SOURCE: &str = include_str!("../resources/core.ocelot");

#[derive(Debug, Clone)]
pub struct Engine {
    pal: PalHandle,
}

impl Engine {
    pub fn new(pal: PalHandle) -> Self {
        Self { pal }
    }

    pub fn run_file(&self, path: impl Into<FilePath>) -> OcelotResult<()> {
        let program = self.compile_program(path.into())?;
        let entry_module = program.entry_module();

        match entry_module.kind {
            SourceFileKind::Script => ocelot_interpreter::interpret_script::interpret_script(
                &entry_module.script,
                &entry_module.source_file,
                &program.environment,
                &*self.pal,
            ),
            SourceFileKind::Module => {
                self.run_module_entrypoint(entry_module, &program.environment)
            }
        }
    }

    pub fn discover_tests(&self, path: impl Into<FilePath>) -> OcelotResult<Vec<DiscoveredTest>> {
        let program = self.compile_program(path.into())?;
        let entry_module = program.entry_module();

        Ok(entry_module
            .script
            .items
            .iter()
            .filter_map(|item| match &item.kind {
                ItemKind::Effect(_) => None,
                ItemKind::Function(_) => None,
                ItemKind::Test(test_item) => Some(DiscoveredTest::new(
                    test_item.name.clone(),
                    test_item.span.clone(),
                )),
                ItemKind::Statement(_) => None,
                ItemKind::Use(_) => None,
            })
            .collect())
    }

    pub fn run_test(&self, path: impl Into<FilePath>, test_name: &str) -> OcelotResult<()> {
        let program = self.compile_program(path.into())?;
        let entry_module = program.entry_module();
        let test_item = entry_module
            .script
            .items
            .iter()
            .find_map(|item| match &item.kind {
                ItemKind::Test(test_item) if test_item.name == test_name => Some(test_item),
                _ => None,
            })
            .context(format!("unknown test `{test_name}`"))?;

        if let Err(error) = ocelot_interpreter::interpreter::Interpreter::new(
            &*self.pal,
            &entry_module.source_file,
            &program.environment,
        )
        .interpret_statements(&test_item.body)
        {
            return match error.kind() {
                ErrorKind::AssertionError(assertion_error) => Err(OcelotError::message(format!(
                    "test `{}` failed\n{}",
                    test_item.name,
                    render_assertion_error(assertion_error)
                ))),
                ErrorKind::RuntimeError(diagnostic) => Err(OcelotError::message(format!(
                    "test `{}` failed\n{}",
                    test_item.name,
                    render_source_diagnostics(std::slice::from_ref(diagnostic.as_ref()))
                ))),
                _ => Err(error).context(format!("test `{}` failed", test_item.name)),
            };
        }

        Ok(())
    }

    pub fn run_tests(&self, path: impl Into<FilePath>) -> OcelotResult<TestRunSummary> {
        let program = self.compile_program(path.into())?;
        let entry_module = program.entry_module();
        let interpreter = ocelot_interpreter::interpreter::Interpreter::new(
            &*self.pal,
            &entry_module.source_file,
            &program.environment,
        );
        let mut summary = TestRunSummary::new();

        for item in &entry_module.script.items {
            let ItemKind::Test(test_item) = &item.kind else {
                continue;
            };

            match interpreter.interpret_statements(&test_item.body) {
                Ok(()) => summary.passed.push(test_item.name.clone()),
                Err(error) => match error.kind() {
                    ErrorKind::AssertionError(assertion_error) => {
                        summary.failed.push(FailedTestResult::new(
                            test_item.name.clone(),
                            format!(
                                "test `{}` failed\n{}",
                                test_item.name,
                                render_assertion_error(assertion_error)
                            ),
                        ))
                    }
                    ErrorKind::RuntimeError(diagnostic) => {
                        summary.failed.push(FailedTestResult::new(
                            test_item.name.clone(),
                            format!(
                                "test `{}` failed\n{}",
                                test_item.name,
                                render_source_diagnostics(std::slice::from_ref(
                                    diagnostic.as_ref()
                                ))
                            ),
                        ))
                    }
                    _ => summary.failed.push(FailedTestResult::new(
                        test_item.name.clone(),
                        OcelotError::message(format!("test `{}` failed", test_item.name))
                            .with_source(error)
                            .to_test_string(),
                    )),
                },
            }
        }

        Ok(summary)
    }

    fn compile_program(&self, entry_path: FilePath) -> OcelotResult<LoadedProgram> {
        let entry_kind = SourceFileKind::from_path(&entry_path)?;
        let execution_root = entry_path.parent().unwrap_or_else(|| FilePath::from(""));
        let mut module_paths = self
            .pal
            .walk_directory(&execution_root, &[String::from("*.ocelot")])?
            .collect::<OcelotResult<Vec<_>>>()?;
        module_paths.retain(|path| path.extension() == Some("ocelot"));
        module_paths.sort();

        if !module_paths.contains(&entry_path) {
            module_paths.push(entry_path.clone());
            module_paths.sort();
        }

        let mut modules = vec![self.load_core_module()?];
        modules.extend(
            module_paths
                .into_iter()
                .map(|path| {
                    let module_kind = if path == entry_path {
                        entry_kind
                    } else {
                        SourceFileKind::Module
                    };
                    self.load_module(&execution_root, path, module_kind)
                })
                .collect::<OcelotResult<Vec<_>>>()?,
        );
        let entry_module_index = modules
            .iter()
            .position(|module| module.source_file.path == entry_path)
            .context("internal error: entry module was not loaded")?;

        let mut compilation_context = CompilationContext::default();
        for module in &modules {
            validate_loaded_module(module, &mut compilation_context);
            validate_reserved_core_module_name(module, &mut compilation_context);
        }
        ocelot_resolver::finish_resolution(&compilation_context)?;

        let native_function_registry = self.create_native_function_registry();
        let mut environment = self.create_program_environment();

        for module in &modules {
            environment.add_module(module.module_name.clone());
        }

        for module in &mut modules {
            ocelot_resolver::register_module_effects(
                &mut module.script,
                &module.source_file,
                &mut compilation_context,
                &mut environment,
            )?;
        }

        for module in &mut modules {
            ocelot_resolver::register_module_functions(
                &mut module.script,
                module.module_name.as_str(),
                &module.source_file,
                &mut compilation_context,
                &mut environment,
                &native_function_registry,
            )?;
        }

        for module in &mut modules {
            ocelot_resolver::register_module_imports(
                &mut module.script,
                module.module_name.as_str(),
                &module.source_file,
                &mut compilation_context,
                &mut environment,
            )?;
        }

        for module in &mut modules {
            ocelot_resolver::resolve_module_items(
                &mut module.script,
                module.module_name.as_str(),
                &module.source_file,
                &mut compilation_context,
                &mut environment,
                &native_function_registry,
            )?;
        }

        ocelot_resolver::resolve_user_defined_function_definitions(
            &mut compilation_context,
            &mut environment,
            &native_function_registry,
        )?;
        ocelot_resolver::finish_resolution(&compilation_context)?;

        Ok(LoadedProgram::new(entry_module_index, modules, environment))
    }

    fn load_module(
        &self,
        execution_root: &FilePath,
        path: FilePath,
        kind: SourceFileKind,
    ) -> OcelotResult<LoadedModule> {
        let source_file = self.load_source_file(path.clone())?;
        let mut compilation_context = CompilationContext::default();
        let script =
            ocelot_parser::parse_script::parse_script(&source_file, &mut compilation_context)?;
        Ok(LoadedModule::new(
            module_name_from_path(execution_root, &path)?,
            kind,
            source_file,
            script,
        ))
    }

    fn load_source_file(&self, path: FilePath) -> OcelotResult<SourceFile> {
        let source = self.pal.read_file_to_string(&path)?;
        Ok(SourceFile::new(path, source))
    }

    fn load_core_module(&self) -> OcelotResult<LoadedModule> {
        let source_file = SourceFile::new(CORE_MODULE_PATH, CORE_MODULE_SOURCE);
        let mut compilation_context = CompilationContext::default();
        let script =
            ocelot_parser::parse_script::parse_script(&source_file, &mut compilation_context)?;
        Ok(LoadedModule::new(
            CORE_MODULE_NAME,
            SourceFileKind::Module,
            source_file,
            script,
        ))
    }

    fn create_program_environment(&self) -> ProgramEnvironment {
        ProgramEnvironment::new()
    }

    fn create_native_function_registry(&self) -> NativeFunctionRegistry {
        default_native_function_registry()
    }

    fn run_module_entrypoint(
        &self,
        entry_module: &LoadedModule,
        environment: &ProgramEnvironment,
    ) -> OcelotResult<()> {
        let entrypoint_name = environment.qualify_function_name(&entry_module.module_name, "main");
        let function_index = environment
            .resolve_function_exact(entrypoint_name.as_str())
            .context(format!(
                "module `{}` does not define a `main()` entrypoint",
                entry_module.module_name
            ))?;
        let function_definition = environment.function_definition(function_index)?;
        let FunctionKind::UserDefined {
            function,
            source_file,
        } = &function_definition.kind
        else {
            ocelot_base::bail!(
                "internal error: module entrypoint `{entrypoint_name}` must be user-defined"
            );
        };

        ocelot_interpreter::interpreter::Interpreter::new(&*self.pal, source_file, environment)
            .interpret_statements(&function.body)
    }
}

fn module_name_from_path(execution_root: &FilePath, path: &FilePath) -> OcelotResult<SharedString> {
    let relative_path = path
        .as_path()
        .strip_prefix(execution_root.as_path())
        .with_context(|| {
            format!("internal error: `{path}` is not inside execution root `{execution_root}`")
        })?;
    let mut relative_path = relative_path.to_path_buf();
    relative_path.set_extension("");

    let segments = relative_path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    Ok(segments.join("::").into())
}

fn validate_loaded_module(module: &LoadedModule, compilation_context: &mut CompilationContext) {
    if module.kind.allows_top_level_statements() {
        return;
    }

    let Some(statement) = module.script.statements().next() else {
        return;
    };

    compilation_context.add_diagnostic(module_statement_diagnostic(
        &module.source_file,
        statement.span.clone(),
    ));
}

fn validate_reserved_core_module_name(
    module: &LoadedModule,
    compilation_context: &mut CompilationContext,
) {
    if module.module_name != CORE_MODULE_NAME
        || module.source_file.path.as_str() == CORE_MODULE_PATH
    {
        return;
    }

    compilation_context.add_diagnostic(
        SourceDiagnostic::new(
            DiagnosticLevel::Error,
            &module.source_file.path,
            "module name `core` is reserved",
        )
        .with_excerpt(source_excerpt_for_path(
            &module.source_file,
            "rename this file or module path",
        )),
    );
}

fn source_excerpt_for_path(
    source_file: &SourceFile,
    annotation: impl Into<SharedString>,
) -> SourceExcerpt {
    let annotation = annotation.into();
    let source_line = source_file.source().lines().next().unwrap_or_default();

    SourceExcerpt::new(&source_file.path, 1, source_line)
        .with_annotation(SourceAnnotation::new(Span::new(0, 0), annotation))
}

fn module_statement_diagnostic(source_file: &SourceFile, span: Span) -> SourceDiagnostic {
    let (line_number, line_start, line_end) = line_bounds(source_file.source(), span.start());
    let source_line = &source_file.source()[line_start..line_end];
    let relative_start = span.start().saturating_sub(line_start);
    let relative_end = span.end().saturating_sub(line_start);

    SourceDiagnostic::new(
        DiagnosticLevel::Error,
        &source_file.path,
        "top-level statements are only allowed in `.ocelot-script` files",
    )
    .with_excerpt(
        SourceExcerpt::new(&source_file.path, line_number, source_line).with_annotation(
            SourceAnnotation::new(
                Span::new(relative_start, relative_end),
                "move this statement into `main()` or rename the file to `.ocelot-script`",
            ),
        ),
    )
}

fn line_bounds(source: &str, index: usize) -> (usize, usize, usize) {
    let line_start = source[..index].rfind('\n').map_or(0, |offset| offset + 1);
    let line_end = source[index..]
        .find('\n')
        .map_or(source.len(), |offset| index + offset);
    let line_number = source[..line_start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;

    (line_number, line_start, line_end)
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use super::module_name_from_path;
    use crate::source_file_kind::SourceFileKind;
    use expect_test::expect;
    use ocelot_base::compilation_stage::CompilationStage;
    use ocelot_base::error::ErrorKind;
    use ocelot_base::file_path::FilePath;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;

    #[test]
    fn derives_nested_module_names_from_paths() {
        assert_eq!(
            module_name_from_path(
                &FilePath::from("examples"),
                &FilePath::from("examples/math/greet.ocelot")
            )
            .unwrap()
            .as_str(),
            "math::greet"
        );
    }

    #[test]
    fn run_file_executes_a_script_file() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/hello_world.ocelot-script",
            "println(\"hello, world\");",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine
            .run_file("examples/hello_world.ocelot-script")
            .unwrap();

        expect![[r#"
            READ FILE: examples/hello_world.ocelot-script
            PRINT: hello, world

        "#]]
        .assert_eq(&pal.get_effects());
        assert_eq!(pal.take_printed_output(), "hello, world\n");
    }

    #[test]
    fn run_file_uses_core_functions_as_a_fallback_only() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            "fun println() { helper::greet(); }\nprintln();",
        );
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet() { core::println(\"local wins\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_file("examples/main.ocelot-script").unwrap();

        assert_eq!(pal.take_printed_output(), "local wins\n");
    }

    #[test]
    fn run_file_prefers_explicit_imports_over_core_fallback() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            "use helper::println;\nprintln();",
        );
        pal.set_file(
            "examples/helper.ocelot",
            "fun println() { core::println(\"import wins\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_file("examples/main.ocelot-script").unwrap();

        assert_eq!(pal.take_printed_output(), "import wins\n");
    }

    #[test]
    fn run_file_executes_functions_from_sibling_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "helper::greet();");
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet() { println(\"hello\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_file("examples/main.ocelot-script").unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn run_file_executes_imported_functions_from_sibling_modules() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            "use helper::greet;\ngreet();",
        );
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet() { println(\"hello\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_file("examples/main.ocelot-script").unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn run_file_executes_grouped_imports() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            "use helper::{greet, wave};\ngreet();\nwave();",
        );
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet() { println(\"hello\"); } fun wave() { println(\"bye\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_file("examples/main.ocelot-script").unwrap();

        assert_eq!(pal.take_printed_output(), "hello\nbye\n");
    }

    #[test]
    fn run_file_executes_functions_from_nested_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "math::greet::hello();");
        pal.set_file(
            "examples/math/greet.ocelot",
            "fun hello() { println(\"hello\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_file("examples/main.ocelot-script").unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn run_file_executes_main_in_a_module_file() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot", "fun main() { helper::greet(); }");
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet() { println(\"hello\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_file("examples/main.ocelot").unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn run_file_ignores_sibling_script_files_when_loading_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "helper::greet();");
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet() { println(\"hello\"); }",
        );
        pal.set_file("examples/side_effect.ocelot-script", "println(\"wrong\");");

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_file("examples/main.ocelot-script").unwrap();

        let effects = pal.get_effects();
        assert!(effects.contains("READ FILE: examples/helper.ocelot"));
        assert!(!effects.contains("READ FILE: examples/side_effect.ocelot-script"));
        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn run_file_rejects_top_level_statements_in_module_files() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "helper::greet();");
        pal.set_file(
            "examples/helper.ocelot",
            "println(\"setup\"); fun greet() { println(\"hello\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine.run_file("examples/main.ocelot-script").unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(
            error
                .to_test_string()
                .contains("top-level statements are only allowed in `.ocelot-script` files")
        );
        assert!(error.to_test_string().contains("examples/helper.ocelot"));
    }

    #[test]
    fn run_file_reports_unknown_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "helper::greet();");

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine.run_file("examples/main.ocelot-script").unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(error.to_test_string().contains("unknown module `helper`"));
    }

    #[test]
    fn run_file_reports_missing_functions_in_loaded_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "helper::greet();");
        pal.set_file("examples/helper.ocelot", "fun wave() {}");

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine.run_file("examples/main.ocelot-script").unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(
            error
                .to_test_string()
                .contains("module `helper` has no function `greet`")
        );
    }

    #[test]
    fn run_file_reports_duplicate_imports() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            "use helper::greet;\nuse helper::greet;",
        );
        pal.set_file("examples/helper.ocelot", "fun greet() {}");

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine.run_file("examples/main.ocelot-script").unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(error.to_test_string().contains("duplicate import `greet`"));
    }

    #[test]
    fn run_file_reports_missing_module_main_function() {
        let pal = PalMock::new();
        pal.set_file("examples/tool.ocelot", "fun helper() {}");

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine.run_file("examples/tool.ocelot").unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("module `tool` does not define a `main()` entrypoint")
        );
    }

    #[test]
    fn run_file_rejects_user_defined_core_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "println(\"hello\");");
        pal.set_file("examples/core.ocelot", "fun helper() {}");

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine.run_file("examples/main.ocelot-script").unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(
            error
                .to_test_string()
                .contains("module name `core` is reserved")
        );
        assert!(error.to_test_string().contains("examples/core.ocelot"));
    }

    #[test]
    fn run_test_executes_only_entry_module_tests() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            "test \"main\" { helper::greet(); }",
        );
        pal.set_file(
            "examples/helper.ocelot",
            "test \"helper\" { println(\"wrong\"); } fun greet() { println(\"hello\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine
            .run_test("examples/main.ocelot-script", "main")
            .unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn run_test_reports_cross_file_runtime_diagnostics_with_the_called_file_path() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            "test \"broken\" { helper::greet(); }",
        );
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet() { println(missing_value); }",
        );

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine
            .run_test("examples/main.ocelot-script", "broken")
            .unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("unresolved identifier `missing_value`")
        );
        assert!(error.to_test_string().contains("examples/helper.ocelot"));
    }

    #[test]
    fn source_file_kind_is_part_of_loaded_modules() {
        assert_eq!(
            SourceFileKind::Module,
            SourceFileKind::from_path(&FilePath::from("x.ocelot")).unwrap()
        );
        assert_eq!(
            SourceFileKind::Script,
            SourceFileKind::from_path(&FilePath::from("x.ocelot-script")).unwrap()
        );
    }
}
