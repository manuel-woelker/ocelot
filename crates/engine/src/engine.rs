use crate::builtin_module::BuiltinModule;
use crate::core_module::default_core_module;
use crate::discovered_test::DiscoveredTest;
use crate::engine_command::EngineCommand;
use crate::engine_worker::EngineWorker;
use crate::test_run_summary::TestRunSummary;
use ocelot_base::error::{ErrorKind, OcelotError};
use ocelot_base::file_path::FilePath;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_base::source_file::SourceFile;
use ocelot_pal::pal::PalHandle;
use ocelot_parser::parse_compilation_unit::parse_compilation_unit;

#[derive(Debug, Clone)]
pub struct Engine {
    pal: PalHandle,
    builtin_modules: Vec<BuiltinModule>,
}

impl Engine {
    pub fn new(pal: PalHandle) -> Self {
        Self {
            pal,
            builtin_modules: vec![default_core_module()],
        }
    }

    pub fn register_builtin_module(
        &mut self,
        module_name: impl Into<SharedString>,
        source: impl Into<SharedString>,
    ) {
        let builtin_module = BuiltinModule::new(module_name, source);
        self.builtin_modules
            .retain(|module| module.module_name != builtin_module.module_name);
        self.builtin_modules.push(builtin_module);
    }

    pub fn run_file(&self, path: impl Into<FilePath>) -> OcelotResult<()> {
        let path = path.into();
        let command = EngineCommand::run_file(path)?;
        let mut worker = EngineWorker::new(&self.pal, command, self.builtin_modules.clone());
        self.run_worker(&mut worker)
    }

    pub fn run_test(&self, path: impl Into<FilePath>, test_name: &str) -> OcelotResult<()> {
        let path = path.into();
        let command = EngineCommand::run_test(path, test_name)?;
        let mut worker = EngineWorker::new(&self.pal, command, self.builtin_modules.clone());
        self.run_worker(&mut worker)
    }

    pub fn run_tests(&self, path: impl Into<FilePath>) -> OcelotResult<TestRunSummary> {
        let path = path.into();
        let command = EngineCommand::run_tests(path)?;
        let mut worker = EngineWorker::new(&self.pal, command, self.builtin_modules.clone());
        self.run_worker(&mut worker)?;
        Ok(worker.test_run_summary().clone())
    }

    pub fn run_named_tests(
        &self,
        path: impl Into<FilePath>,
        test_names: Vec<SharedString>,
    ) -> OcelotResult<TestRunSummary> {
        let path = path.into();
        let command = EngineCommand::run_named_tests(path, test_names)?;
        let mut worker = EngineWorker::new(&self.pal, command, self.builtin_modules.clone());
        self.run_worker(&mut worker)?;
        Ok(worker.test_run_summary().clone())
    }

    pub fn discover_tests(&self, path: impl Into<FilePath>) -> OcelotResult<Vec<DiscoveredTest>> {
        let path = path.into();
        let source = self.pal.read_file_to_string(&path)?;
        let source_file = SourceFile::new(path.clone(), source);
        let mut source_diagnostics = SourceDiagnostics::default();
        let compilation_unit =
            match parse_compilation_unit(&source_file, &mut source_diagnostics) {
                Ok(compilation_unit) => compilation_unit,
                Err(error) if source_diagnostics.diagnostics.is_empty() => return Err(error),
                Err(error) => {
                    return Err(error.with_source(OcelotError::message(
                        render_source_diagnostics(&source_diagnostics.diagnostics),
                    )));
                }
            };

        Ok(compilation_unit
            .items
            .iter()
            .filter_map(|item| match &item.kind {
                ocelot_ast::item_kind::ItemKind::Test(test_item) => Some(DiscoveredTest::new(
                    path.clone(),
                    test_item.name.clone(),
                    test_item.span.clone(),
                )),
                _ => None,
            })
            .collect())
    }

    fn run_worker(&self, worker: &mut EngineWorker) -> OcelotResult<()> {
        worker
            .run_command()
            .map_err(|error| attach_compilation_diagnostics(error, worker))
    }
}

fn attach_compilation_diagnostics(error: OcelotError, worker: &EngineWorker) -> OcelotError {
    let ErrorKind::CompilationError(_) = error.kind() else {
        return error;
    };

    if worker.source_diagnostics().diagnostics.is_empty() {
        return error;
    }

    error.with_source(OcelotError::message(render_source_diagnostics(
        &worker.source_diagnostics().diagnostics,
    )))
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use crate::module_name_from_path::module_name_from_path;
    use crate::passed_test_result::PassedTestResult;
    use expect_test::expect;
    use ocelot_base::compilation_stage::CompilationStage;
    use ocelot_base::error::ErrorKind;
    use ocelot_base::file_path::FilePath;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;
    use ocelot_semantic::source_file_kind::SourceFileKind;

    #[test]
    fn bindings_infer_types_and_restore_outer_names_after_blocks_and_branches() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            r#"
let name = "outer";
let saved = { println("initialized once"); "saved" };
println(saved);
println(saved);
{
    let name = "${name} inner";
    let enabled: bool = true;
    let message: string = if enabled { name } else { "wrong" };
    println(message);
};
if true { let name = "branch"; println(name); };
println(name);
let empty: () = {};
assert_eq(expected: empty, actual: { true; });
fun greeting(name: string) -> string { let message = "Hello ${name}!"; message }
println(greeting(name: "Ada"));
println(greeting(name: "Grace"));
let println = false;
core::println("qualified");
"#,
        );
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(
            pal.take_printed_output(),
            "initialized once\nsaved\nsaved\nouter inner\nbranch\nouter\nHello Ada!\nHello Grace!\nqualified\n"
        );
    }

    #[test]
    fn bindings_do_not_leak_across_calls_tests_or_initializer_blocks() {
        let pal = PalMock::new();
        pal.set_file("examples/helper.ocelot", "fun greet(name: string) -> string { let result = { let name = \"${name}!\"; name }; result }");
        pal.set_file(
            "examples/main.ocelot-script",
            r#"
use helper::greet;
let root = "script";
println(greet(name: root));
println(greet(name: "second"));
test "first" { let root = "test one"; assert_eq(expected: "test one", actual: root); }
test "second" { let root = "test two"; assert_eq(expected: "test two", actual: root); }
"#,
        );
        let engine = Engine::new(PalHandle::new(pal.clone()));
        engine.run_file("examples/main.ocelot-script").unwrap();
        assert_eq!(pal.take_printed_output(), "script!\nsecond!\n");
        let summary = engine.run_tests("examples/main.ocelot-script").unwrap();
        assert_eq!(summary.passed.len(), 2);
        assert!(summary.failed.is_empty());
        assert_eq!(pal.take_printed_output(), "");
    }

    #[test]
    fn invalid_bindings_fail_before_running_any_effects() {
        for (source, expected) in [
            (
                "println(later); let later = true;",
                "unresolved identifier `later`",
            ),
            ("let value = value;", "unresolved identifier `value`"),
            (
                "let value = true; let value = false;",
                "duplicate binding `value`",
            ),
            (
                "fun f(name: string) { let name = \"duplicate\"; }",
                "duplicate binding `name`",
            ),
            (
                "{ let local = true; }; assert(local);",
                "unresolved identifier `local`",
            ),
            (
                "if true { let local = true; }; assert(local);",
                "unresolved identifier `local`",
            ),
            (
                "let value = { let inside = true; inside }; assert(inside);",
                "unresolved identifier `inside`",
            ),
            (
                "if { let condition = true; condition } { assert(condition); };",
                "unresolved identifier `condition`",
            ),
            ("let flag: bool = \"bad\";", "binding type mismatch"),
            (
                "let flag: number = true;",
                "unsupported binding type `number`",
            ),
            ("let flag: any = true;", "unsupported binding type `any`"),
            ("let flag: () = false;", "binding type mismatch"),
            ("let value = false; value();", "is a value, not a function"),
            (
                "let println = false; println(\"bad\");",
                "is a value, not a function",
            ),
            (
                "let script = \"value\"; fun f() { println(script); }",
                "unresolved identifier `script`",
            ),
            (
                "let script = true; test \"isolated\" { assert(script); }",
                "unresolved identifier `script`",
            ),
            (
                "test \"one\" { let local = true; } test \"two\" { assert(local); }",
                "unresolved identifier `local`",
            ),
            (
                "fun f() cannot write_stdout { let output = println(\"bad\"); }",
                "cannot perform effect",
            ),
        ] {
            let pal = PalMock::new();
            pal.set_file(
                "examples/main.ocelot-script",
                format!("println(\"must not run\"); {source}"),
            );
            let error = Engine::new(PalHandle::new(pal.clone()))
                .run_file("examples/main.ocelot-script")
                .unwrap_err();
            assert!(
                matches!(
                    error.kind(),
                    ErrorKind::CompilationError(CompilationStage::Resolver)
                ),
                "{source}"
            );
            assert!(
                error.to_test_string().contains(expected),
                "{source}\n{error:?}"
            );
            assert_eq!(pal.take_printed_output(), "", "{source}");
        }
    }

    #[test]
    fn a_returning_initializer_exits_before_binding_or_subsequent_effects() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "fun early() -> string { let impossible: bool = { return \"early\"; }; println(\"wrong\"); } println(early());");
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(pal.take_printed_output(), "early\n");
    }

    #[test]
    fn conditionals_select_only_one_branch_and_work_as_discarded_statements() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/helper.ocelot",
            "fun select(enabled: bool) -> string { if enabled { \"yes\" } else { \"no\" } }",
        );
        pal.set_file("examples/main.ocelot-script", r#"
use helper::select;
fun condition() -> bool { println("condition"); true }
println(if condition() { println("then"); select(enabled: true) } else { println("wrong"); "wrong" });
if false { assert(false); } else { println("else"); };
if true { "discarded" } else { "discarded too" };
println("after");
assert(if false { true } else { true });
"#);
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(
            pal.take_printed_output(),
            "condition\nthen\nyes\nelse\nafter\n"
        );
    }

    #[test]
    fn conditional_returns_cover_both_paths_and_propagate_from_conditions() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", r#"
fun both(flag: bool) -> string { if flag { return "then"; } else { return "else"; }; println("wrong"); }
fun mixed(flag: bool) -> string { if flag { return "returned"; } else { "tail" } }
fun fallback(flag: bool) -> string { if flag { return "early"; }; "later" }
fun from_condition() -> string { if { return "condition return"; } {} else {}; println("wrong"); }
fun argument() -> string { assert_eq(expected: if false { return "wrong"; } else { return "argument return"; }, actual: println("wrong")); }
println(both(flag: true));
println(both(flag: false));
println(mixed(flag: true));
println(mixed(flag: false));
println(fallback(flag: true));
println(fallback(flag: false));
println(from_condition());
println(argument());
"#);
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(
            pal.take_printed_output(),
            "then\nelse\nreturned\ntail\nearly\nlater\ncondition return\nargument return\n"
        );
    }

    #[test]
    fn invalid_conditionals_check_all_paths_before_running_effects() {
        for (source, expected) in [
            ("if \"truthy\" {} ;", "if condition must be bool"),
            ("if println(\"wrong\") {};", "if condition must be bool"),
            (
                "if true { true } else { \"no\" };",
                "if branches must have matching types",
            ),
            (
                "if true { \"value\" };",
                "if branches must have matching types",
            ),
            (
                "if true {} else if false { false };",
                "if branches must have matching types",
            ),
            (
                "fun f(flag: bool) -> bool { if flag { return true; }; }",
                "block result type mismatch",
            ),
            (
                "fun f() -> bool { if true { return true; }; }",
                "block result type mismatch",
            ),
            ("if true {} else { missing; };", "unresolved identifier"),
            (
                "fun f() -> bool { if false { return \"bad\"; } else { true } }",
                "return type mismatch",
            ),
            (
                "fun f() cannot write_stdout { if false { println(\"never\"); }; }",
                "cannot perform effect",
            ),
            ("if false { return; };", "return is only allowed"),
        ] {
            let pal = PalMock::new();
            pal.set_file(
                "examples/main.ocelot-script",
                format!("println(\"must not run\"); {source}"),
            );
            let error = Engine::new(PalHandle::new(pal.clone()))
                .run_file("examples/main.ocelot-script")
                .unwrap_err();
            assert!(
                matches!(
                    error.kind(),
                    ErrorKind::CompilationError(CompilationStage::Resolver)
                ),
                "{source}"
            );
            assert!(
                error.to_test_string().contains(expected),
                "{source}\n{error:?}"
            );
            assert_eq!(pal.take_printed_output(), "", "{source}");
        }
    }

    #[test]
    fn nested_blocks_keep_value_order_across_imported_calls() {
        let pal = PalMock::new();
        pal.set_file("examples/helper.ocelot", "fun mark(positional value: string) -> string { println(value); { value } } fun combine(first: string, second: string) -> string { \"${first} ${second}\" }");
        pal.set_file(
            "examples/main.ocelot-script",
            r#"
use helper::mark;
println(helper::combine(first: { mark("first") }, second: { mark("second") }));
fun flag() -> bool { { true } }
assert(flag());
assert_eq(expected: {}, actual: { false; });
"#,
        );
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(pal.take_printed_output(), "first\nsecond\nfirst second\n");
    }

    #[test]
    fn returns_propagate_through_every_expression_context_and_stop_later_effects() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", r#"
fun from_argument() -> string { assert_eq(expected: { return "argument"; }, actual: println("bad")); "bad" }
fun from_assert() -> string { assert({ return "assert"; }); "bad" }
fun sink(value: bool) { println("bad"); }
fun from_user_argument() -> string { sink(value: { return "user argument"; }); "bad" }
fun from_not() -> bool { not { return false; } }
fun from_template() -> string { "prefix ${{ return "template"; }} ${println("bad")}" }
fun from_return() -> string { return { return "inner"; }; }
fun inner() -> string { return "called"; }
fun caller() -> string { { inner(); }; "caller" }
println(from_argument());
println(from_assert());
println(from_user_argument());
assert_eq(expected: false, actual: from_not());
println(from_template());
println(from_return());
fun consumes() -> string { { return "nested"; }; println("bad"); }
println(consumes());
println(caller());
"#);
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(
            pal.take_printed_output(),
            "argument\nassert\nuser argument\ntemplate\ninner\nnested\ncaller\n"
        );
    }

    #[test]
    fn block_results_do_not_weaken_type_name_or_effect_checks() {
        let cases = [
            ("fun f() { \"value\" }", "block result type mismatch"),
            (
                "fun f() -> bool { \"value\" }",
                "block result type mismatch",
            ),
            (
                "fun f() -> string { \"discarded\"; }",
                "block result type mismatch",
            ),
            ("assert({ \"value\" });", "expects a bool argument"),
            (
                "fun f() -> bool { { return \"wrong\"; } }",
                "return type mismatch",
            ),
            (
                "fun f() -> bool { { return true; }; missing; }",
                "unresolved identifier",
            ),
            (
                "fun f() -> bool cannot write_stdout { { return true; }; println(\"bad\"); }",
                "cannot perform effect",
            ),
            ("test \"value\" { true }", "test bodies must produce unit"),
            ("{ return; };", "return is only allowed"),
        ];
        for (source, expected) in cases {
            let pal = PalMock::new();
            pal.set_file(
                "examples/main.ocelot-script",
                format!("println(\"must not run\"); {source}"),
            );
            let error = Engine::new(PalHandle::new(pal.clone()))
                .run_file("examples/main.ocelot-script")
                .unwrap_err();
            assert!(
                error.to_test_string().contains(expected),
                "{source}\n{error:?}"
            );
            assert_eq!(pal.take_printed_output(), "", "{source}");
        }
    }

    #[test]
    fn typed_calls_compose_across_imports_and_return_before_later_effects() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/helper.ocelot",
            r#"
fun greeting(positional name: string) -> string {
    return "Hello ${name}!";
    println("unreachable");
}
fun invert(value: bool) -> bool { return not value; }
"#,
        );
        pal.set_file(
            "examples/main.ocelot-script",
            r#"
use helper::greeting;
println(greeting("Ada"));
assert(helper::invert(value: false));
assert_eq(expected: "Hello Grace!", actual: greeting(name: "Grace"));
fun forward() -> string { return later(); }
fun later() -> string { return "forward"; }
fun recursive() -> bool { return recursive(); }
println(forward());
fun finish() -> () { return; println("unreachable"); }
fun via_native() { return println("native unit"); }
finish();
via_native();
println("caller continues");
"#,
        );
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(
            pal.take_printed_output(),
            "Hello Ada!\nforward\nnative unit\ncaller continues\n"
        );
    }

    #[test]
    fn returned_argument_values_keep_source_order_and_do_not_exit_the_caller() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            r#"
fun mark(positional value: string) -> string { println(value); return value; }
fun combine(left: string, right: string) -> string { return "${left} ${right}"; }
println(combine(left: mark("first"), right: mark("second")));
println("after");
"#,
        );
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(
            pal.take_printed_output(),
            "first\nsecond\nfirst second\nafter\n"
        );
    }

    #[test]
    fn invalid_returns_and_typed_call_uses_fail_before_effects() {
        let cases = [
            (
                "fun f() -> string {}",
                "block result type mismatch: expected string, got ()",
            ),
            (
                "fun f() -> bool { true; }",
                "block result type mismatch: expected bool, got ()",
            ),
            (
                "fun f() -> string { return true; }",
                "return type mismatch: expected string, got bool",
            ),
            (
                "fun f() -> bool { return; }",
                "return type mismatch: expected bool, got ()",
            ),
            (
                "fun f() { return \"value\"; }",
                "return type mismatch: expected (), got string",
            ),
            (
                "fun f() -> () { return false; }",
                "return type mismatch: expected (), got bool",
            ),
            (
                "fun f() -> number { return true; }",
                "unsupported return type `number`",
            ),
            (
                "fun f() -> any { return true; }",
                "unsupported return type `any`",
            ),
            ("return;", "return is only allowed in function bodies"),
            (
                "test \"f\" { return true; }",
                "return is only allowed in function bodies",
            ),
            (
                "fun f() -> string { return missing; }",
                "unresolved identifier `missing`",
            ),
            (
                "fun f() -> string { return \"ok\"; missing; }",
                "unresolved identifier `missing`",
            ),
            (
                "fun f() -> bool { return true; return \"bad\"; }",
                "return type mismatch: expected bool, got string",
            ),
            (
                "fun f() -> string { return \"bad\"; } assert(f());",
                "expects a bool argument",
            ),
            (
                "fun f() -> string { return \"bad\"; } not f();",
                "operator `not` expects a bool operand",
            ),
            (
                "fun unit() {} fun f() -> bool { return unit(); }",
                "return type mismatch: expected bool, got ()",
            ),
        ];
        for (source, expected) in cases {
            let pal = PalMock::new();
            pal.set_file(
                "examples/main.ocelot-script",
                format!("println(\"must not run\"); {source}"),
            );
            let error = Engine::new(PalHandle::new(pal.clone()))
                .run_file("examples/main.ocelot-script")
                .unwrap_err();
            assert!(
                matches!(
                    error.kind(),
                    ErrorKind::CompilationError(CompilationStage::Resolver)
                ),
                "{source}"
            );
            let rendered = ocelot_base::cli::format_cli_error("failed", &error);
            assert!(rendered.contains(expected), "{source}\n{rendered}");
            assert_eq!(pal.take_printed_output(), "", "{source}");
        }
    }

    #[test]
    fn returned_calls_and_unreachable_statements_still_contribute_effects() {
        for body in ["return println(\"bad\");", "return; println(\"bad\");"] {
            let pal = PalMock::new();
            pal.set_file(
                "examples/main.ocelot-script",
                format!("fun f() -> () cannot write_stdout {{ {body} }} f();"),
            );
            let error = Engine::new(PalHandle::new(pal.clone()))
                .run_file("examples/main.ocelot-script")
                .unwrap_err();
            assert!(
                error
                    .to_test_string()
                    .contains("cannot perform effect `write_stdout`")
            );
            assert_eq!(pal.take_printed_output(), "");
        }
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "fun noisy() -> bool can write_stdout { return true; } fun caller() -> bool cannot write_stdout { return noisy(); }");
        let error = Engine::new(PalHandle::new(pal))
            .run_file("examples/main.ocelot-script")
            .unwrap_err();
        assert!(
            error
                .to_test_string()
                .contains("cannot perform effect `write_stdout`")
        );
    }

    #[test]
    fn native_result_annotations_match_registered_signatures() {
        let core = include_str!("../resources/builtin/core.ocelot");
        for (result, valid) in [("()", true), ("string", false)] {
            let pal = PalMock::new();
            pal.set_file("examples/main.ocelot-script", "println(\"ok\");");
            let mut engine = Engine::new(PalHandle::new(pal.clone()));
            engine.register_builtin_module(
                "core",
                core.replace(
                    ") can write_stdout",
                    &format!(") -> {result} can write_stdout"),
                ),
            );
            let outcome = engine.run_file("examples/main.ocelot-script");
            if valid {
                outcome.unwrap();
                assert_eq!(pal.take_printed_output(), "ok\n");
            } else {
                assert!(
                    outcome
                        .unwrap_err()
                        .to_test_string()
                        .contains("return type does not match its registered signature")
                );
                assert_eq!(pal.take_printed_output(), "");
            }
        }
    }

    #[test]
    fn direct_module_execution_discards_its_return_value_and_stops_at_return() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot", "fun main() -> string { println(\"before\"); return \"ignored\"; println(\"unreachable\"); }");
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot")
            .unwrap();
        assert_eq!(pal.take_printed_output(), "before\n");
    }

    #[test]
    fn unknown_values_fail_during_resolution_before_any_effects() {
        for snippet in [
            "missing;",
            "println(missing);",
            "println(\"${missing}\");",
            "not missing;",
            "fun unused() { println(missing); }",
            "fun f(name: string) { println(other); }",
            "test \"unused\" { println(missing); }",
            "core::println;",
        ] {
            let pal = PalMock::new();
            pal.set_file(
                "examples/main.ocelot-script",
                format!("println(\"must not run\"); {snippet}"),
            );
            let error = Engine::new(PalHandle::new(pal.clone()))
                .run_file("examples/main.ocelot-script")
                .unwrap_err();
            assert!(
                matches!(
                    error.kind(),
                    ErrorKind::CompilationError(CompilationStage::Resolver)
                ),
                "{snippet}"
            );
            let rendered = ocelot_base::cli::format_cli_error("failed", &error);
            assert!(
                rendered.contains("unresolved identifier"),
                "{snippet}: {rendered}"
            );
            assert!(rendered.contains("examples/main.ocelot-script:1:"));
            assert_eq!(pal.take_printed_output(), "", "{snippet}");
        }
    }

    #[test]
    fn calls_preserve_labels_and_omission_policy_across_module_boundaries() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet(positional name: string, excited: bool) { println(\"${name} ${excited}\"); }",
        );
        pal.set_file(
            "examples/main.ocelot-script",
            r#"use helper::greet;
greet("Ada", excited: true);
greet(name: "Grace", excited: false);
helper::greet("Linus", excited: true);
core::println(value: "explicit");
core::assert(condition: true);
assert_eq(expected: true, actual: true);"#,
        );
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(
            pal.take_printed_output(),
            "Ada true\nGrace false\nLinus true\nexplicit\n"
        );
    }

    #[test]
    fn user_call_values_evaluate_left_to_right_before_binding() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            r#"show(first: "${println("first")}", second: "${println("second")}");
fun show(first: string, second: string) { println("${first} ${second}"); }"#,
        );
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(pal.take_printed_output(), "first\nsecond\n() ()\n");
    }

    #[test]
    fn labels_can_forward_local_values_and_all_positional_parameters_allow_omission() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            r#"fun forward(name: string) { show(name: name, excited: true); }
fun show(positional name: string, positional excited: bool) { println(value: "${name} ${excited}"); }
forward(name: "Ada");
show("Grace", false);
show("Linus", excited: true);"#,
        );
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(
            pal.take_printed_output(),
            "Ada true\nGrace false\nLinus true\n"
        );
    }

    #[test]
    fn invalid_argument_labels_and_policies_fail_before_effects_run() {
        let cases = [
            (
                "fun f(name: string) {} f(\"${println(\"bad\")}\");",
                "argument `name` to `f` requires a label",
            ),
            (
                "fun f(name: string) {} f(wrong: \"Ada\");",
                "unknown argument label `wrong`",
            ),
            (
                "fun f(first: string, second: string) {} f(first: \"a\", first: \"b\");",
                "duplicate argument label `first`",
            ),
            (
                "fun f(first: string, second: string) {} f(second: \"a\", first: \"b\");",
                "must follow declaration order",
            ),
            (
                "fun f(positional first: string, positional second: string) {} f(first: \"a\", \"b\");",
                "unlabeled arguments must precede labeled arguments",
            ),
            (
                "fun f(first: string, positional second: string) {}",
                "positional parameters must precede",
            ),
            (
                "fun f(enabled: bool) {} f(enabled: \"bad\");",
                "argument 1 to `f` must be bool",
            ),
            (
                "fun f(name: string) {} f();",
                "expects exactly one argument",
            ),
            ("fun f() {} f(name: \"Ada\");", "expects no arguments"),
            (
                "assert_eq(true, true);",
                "argument `expected` to `assert_eq` requires a label",
            ),
            ("println(wrong: \"bad\");", "unknown argument label `wrong`"),
            ("assert(condition: \"bad\");", "expects a bool argument"),
        ];
        for (source, expected) in cases {
            let pal = PalMock::new();
            pal.set_file("examples/main.ocelot-script", source);
            let error = Engine::new(PalHandle::new(pal.clone()))
                .run_file("examples/main.ocelot-script")
                .unwrap_err();
            assert!(
                matches!(
                    error.kind(),
                    ErrorKind::CompilationError(CompilationStage::Resolver)
                ),
                "{source}"
            );
            let rendered = ocelot_base::cli::format_cli_error("execution failed", &error);
            assert!(rendered.contains(expected), "{source}\n{rendered}");
            assert!(
                rendered.contains("examples/main.ocelot-script:"),
                "{rendered}"
            );
            assert_eq!(pal.take_printed_output(), "", "{source}");
        }
    }

    #[test]
    fn executes_modules_and_embedded_calls_under_an_absolute_filesystem_root() {
        let root =
            std::env::temp_dir().join(format!("ocelot-engine-review-tests-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("main.ocelot"), "fun main() { helper::check(); }").unwrap();
        std::fs::write(root.join("helper.ocelot"), "fun check() { assert(true); }").unwrap();
        let pal = ocelot_pal::pal_real::PalReal::new_handle().unwrap();
        let execution = Engine::new(pal.clone()).run_file(FilePath::new(root.join("main.ocelot")));
        let mut embedded = crate::embedded_engine::EmbeddedEngine::new(pal, FilePath::new(&root));
        let evaluation = embedded.eval("helper::check()");
        std::fs::remove_dir_all(&root).unwrap();
        execution.unwrap();
        assert_eq!(
            evaluation.unwrap(),
            ocelot_semantic::runtime_value::RuntimeValue::unit()
        );
    }

    #[test]
    fn nested_unit_calls_are_type_checked_before_side_effects() {
        for source in [
            "fun accept(value: bool) {} accept(value: println(\"side effect\"));",
            "fun accept(value: string) {} fun empty() {} accept(value: empty());",
            "assert(println(\"side effect\"));",
            "not println(\"side effect\");",
        ] {
            let pal = PalMock::new();
            pal.set_file("examples/main.ocelot-script", source);
            let engine = Engine::new(PalHandle::new(pal.clone()));
            let error = engine.run_file("examples/main.ocelot-script").unwrap_err();
            assert!(
                matches!(
                    error.kind(),
                    ErrorKind::CompilationError(CompilationStage::Resolver)
                ),
                "{source}"
            );
            assert_eq!(pal.take_printed_output(), "", "{source}");
        }
    }

    #[test]
    fn unit_calls_remain_valid_for_any_parameters() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            "assert_eq(expected: println(\"first\"), actual: println(\"second\"));",
        );
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(pal.take_printed_output(), "first\nsecond\n");
    }

    #[test]
    fn module_entrypoint_rejects_parameters_before_side_effects() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot",
            "fun main(name: string) { println(\"must not run\"); println(name); }",
        );
        let error = Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot")
            .unwrap_err();
        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        let rendered = ocelot_base::cli::format_cli_error("execution failed", &error);
        assert!(rendered.contains("must have no parameters"));
        assert!(rendered.contains("examples/main.ocelot:1:5"));
        assert!(rendered.contains("remove parameters from this entrypoint"));
        assert!(!rendered.contains("engine_worker.rs"));
        assert_eq!(pal.take_printed_output(), "");
    }

    #[test]
    fn ordinary_calls_to_parameterized_main_are_valid() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            "fun main(name: string) { println(name); } main(name: \"hello\");",
        );
        Engine::new(PalHandle::new(pal.clone()))
            .run_file("examples/main.ocelot-script")
            .unwrap();
        assert_eq!(pal.take_printed_output(), "hello\n");
    }

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
    fn run_file_executes_template_strings_end_to_end() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot-script",
            "use helper::greet;\ngreet(name: \"Ada\");",
        );
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet(name: string) { println(\"Hello ${name} ${not false}!\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_file("examples/main.ocelot-script").unwrap();

        assert_eq!(pal.take_printed_output(), "Hello Ada true!\n");
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
    fn run_file_supports_registering_additional_builtin_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "helpers::greet();");

        let mut engine = Engine::new(PalHandle::new(pal.clone()));
        engine.register_builtin_module(
            "helpers",
            "fun greet() { core::println(\"from builtin\"); }",
        );

        engine.run_file("examples/main.ocelot-script").unwrap();

        assert_eq!(pal.take_printed_output(), "from builtin\n");
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
    fn run_test_reports_cross_file_resolution_diagnostics_with_the_called_file_path() {
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

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(
            error
                .to_test_string()
                .contains("unresolved identifier `missing_value`")
        );
        assert!(error.to_test_string().contains("examples/helper.ocelot"));
    }

    #[test]
    fn discover_tests_returns_test_names_and_entry_file_path() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/tests.ocelot-script",
            "test \"first\" { println(\"one\"); }\nfun helper() {}\ntest \"second\" { println(\"two\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal));

        let tests = engine
            .discover_tests("examples/tests.ocelot-script")
            .unwrap();

        assert_eq!(tests.len(), 2);
        assert_eq!(
            tests
                .iter()
                .map(|test| test.file_path.as_str())
                .collect::<Vec<_>>(),
            vec![
                "examples/tests.ocelot-script",
                "examples/tests.ocelot-script"
            ]
        );
        assert_eq!(
            tests
                .iter()
                .map(|test| test.name.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        assert!(
            tests
                .iter()
                .all(|test| test.span.start() <= test.span.end())
        );
    }

    #[test]
    fn run_named_tests_executes_only_the_selected_tests() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/tests.ocelot-script",
            "test \"first\" { println(\"one\"); }\ntest \"second\" { println(\"two\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        let summary = engine
            .run_named_tests("examples/tests.ocelot-script", vec!["second".into()])
            .unwrap();

        assert_eq!(
            summary.passed,
            vec![PassedTestResult::new(
                "examples/tests.ocelot-script",
                "second"
            )]
        );
        assert!(summary.failed.is_empty());
        assert_eq!(pal.take_printed_output(), "two\n");
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
