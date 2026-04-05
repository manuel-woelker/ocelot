use crate::engine_worker::EngineWorker;
use crate::test_run_summary::TestRunSummary;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_pal::pal::PalHandle;

#[derive(Debug, Clone)]
pub struct Engine {
    pal: PalHandle,
}

impl Engine {
    pub fn new(pal: PalHandle) -> Self {
        Self { pal }
    }

    pub fn run_file(&self, path: impl Into<FilePath>) -> OcelotResult<()> {
        let worker = EngineWorker::new(&self.pal);
        worker.run_file(path)?;
        Ok(())
    }

    pub fn run_test(&self, path: impl Into<FilePath>, test_name: &str) -> OcelotResult<()> {
        let worker = EngineWorker::new(&self.pal);
        worker.run_test(path, test_name)?;
        Ok(())
    }

    pub fn run_tests(&self, path: impl Into<FilePath>) -> OcelotResult<TestRunSummary> {
        let worker = EngineWorker::new(&self.pal);
        worker.run_tests(path)
    }
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use crate::module_name_from_path::module_name_from_path;
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
