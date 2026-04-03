use crate::discovered_test::DiscoveredTest;
use crate::failed_test_result::FailedTestResult;
use crate::test_run_summary::TestRunSummary;
use ocelot_ast::item_kind::ItemKind;
use ocelot_base::error::OcelotError;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::result::OptionExt;
use ocelot_base::result::ResultExt;
use ocelot_base::source_file::SourceFile;
use ocelot_pal::pal::PalHandle;

#[derive(Debug, Clone)]
pub struct Engine {
    pal: PalHandle,
}

impl Engine {
    pub fn new(pal: PalHandle) -> Self {
        Self { pal }
    }

    pub fn run_script(&self, path: impl Into<FilePath>) -> OcelotResult<()> {
        let source_file = self.load_source_file(path.into())?;
        let script = self.parse_script(&source_file)?;
        ocelot_interpreter::interpret_script::interpret_script(&script, &*self.pal)?;
        Ok(())
    }

    pub fn discover_tests(&self, path: impl Into<FilePath>) -> OcelotResult<Vec<DiscoveredTest>> {
        let source_file = self.load_source_file(path.into())?;
        let script = self.parse_script(&source_file)?;

        Ok(script
            .items
            .into_iter()
            .filter_map(|item| match item.kind {
                ItemKind::Test(test_item) => {
                    Some(DiscoveredTest::new(test_item.name, test_item.span))
                }
                ItemKind::Statement(_) => None,
            })
            .collect())
    }

    pub fn run_test(&self, path: impl Into<FilePath>, test_name: &str) -> OcelotResult<()> {
        let source_file = self.load_source_file(path.into())?;
        let script = self.parse_script(&source_file)?;
        let test_item = script
            .items
            .iter()
            .find_map(|item| match &item.kind {
                ItemKind::Test(test_item) if test_item.name == test_name => Some(test_item),
                _ => None,
            })
            .context(format!("unknown test `{test_name}`"))?;

        ocelot_interpreter::interpreter::Interpreter::new(&*self.pal)
            .interpret_statements(&test_item.body)
            .context(format!("test `{}` failed", test_item.name))?;

        Ok(())
    }

    pub fn run_tests(&self, path: impl Into<FilePath>) -> OcelotResult<TestRunSummary> {
        let source_file = self.load_source_file(path.into())?;
        let script = self.parse_script(&source_file)?;
        let interpreter = ocelot_interpreter::interpreter::Interpreter::new(&*self.pal);
        let mut summary = TestRunSummary::new();

        for item in &script.items {
            let ItemKind::Test(test_item) = &item.kind else {
                continue;
            };

            match interpreter.interpret_statements(&test_item.body) {
                Ok(()) => summary.passed.push(test_item.name.clone()),
                Err(error) => summary.failed.push(FailedTestResult::new(
                    test_item.name.clone(),
                    OcelotError::message(format!("test `{}` failed", test_item.name))
                        .with_source(error)
                        .to_test_string(),
                )),
            }
        }

        Ok(summary)
    }

    fn load_source_file(&self, path: FilePath) -> OcelotResult<SourceFile> {
        let source = self.pal.read_file_to_string(&path)?;
        Ok(SourceFile::new(path, source))
    }

    fn parse_script(&self, source_file: &SourceFile) -> OcelotResult<ocelot_ast::script::Script> {
        let mut compilation_context =
            ocelot_base::compilation_context::CompilationContext::default();
        ocelot_parser::parse_script::parse_script(source_file, &mut compilation_context)
    }
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use expect_test::expect;
    use ocelot_base::compilation_stage::CompilationStage;
    use ocelot_base::error::ErrorKind;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;

    #[test]
    fn run_script_reads_and_executes_a_file() {
        let pal = PalMock::new();
        pal.set_file("examples/hello_world.ocelot", "println(\"hello, world\");");

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_script("examples/hello_world.ocelot").unwrap();

        expect![[r#"
            READ FILE: examples/hello_world.ocelot
            PRINT: hello, world

        "#]]
        .assert_eq(&pal.get_effects());
        assert_eq!(pal.take_printed_output(), "hello, world\n");
    }

    #[test]
    fn run_script_ignores_test_items() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/mixed.ocelot",
            "println(\"setup\"); test \"prints hello\" { println(\"hello\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_script("examples/mixed.ocelot").unwrap();

        assert_eq!(pal.take_printed_output(), "setup\n");
    }

    #[test]
    fn run_script_discards_non_call_expression_statements() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/expressions.ocelot",
            "\"hello\"; println(\"world\");",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_script("examples/expressions.ocelot").unwrap();

        assert_eq!(pal.take_printed_output(), "world\n");
    }

    #[test]
    fn discover_tests_returns_names_in_source_order() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/tests.ocelot",
            "test \"first\" { println(\"one\"); } test \"second\" { println(\"two\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal));

        let tests = engine.discover_tests("examples/tests.ocelot").unwrap();

        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].name, "first");
        assert_eq!(tests[1].name, "second");
    }

    #[test]
    fn run_test_executes_only_the_named_test() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/tests.ocelot",
            "test \"first\" { println(\"one\"); } test \"second\" { println(\"two\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_test("examples/tests.ocelot", "second").unwrap();

        assert_eq!(pal.take_printed_output(), "two\n");
    }

    #[test]
    fn run_test_reports_the_failing_test_name() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/tests.ocelot",
            "test \"broken\" { println(missing_value); }",
        );

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine
            .run_test("examples/tests.ocelot", "broken")
            .unwrap_err();

        assert!(error.to_test_string().contains("test `broken` failed"));
    }

    #[test]
    fn run_tests_reports_unknown_test_names() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/tests.ocelot",
            "test \"first\" { println(\"one\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine
            .run_test("examples/tests.ocelot", "missing")
            .unwrap_err();

        assert!(error.to_test_string().contains("unknown test `missing`"));
    }

    #[test]
    fn run_script_tags_parser_failures_as_compilation_errors() {
        let pal = PalMock::new();
        pal.set_file("examples/broken.ocelot", "println();");

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine.run_script("examples/broken.ocelot").unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Parser)
        ));
        assert!(
            error
                .to_test_string()
                .contains("type error: `println` expects exactly one argument")
        );
    }

    #[test]
    fn run_script_reports_unknown_native_functions() {
        let pal = PalMock::new();
        pal.set_file("examples/broken.ocelot", "printline(\"hello\");");

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine.run_script("examples/broken.ocelot").unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("unknown native function `printline`")
        );
    }

    #[test]
    fn run_tests_reports_passes_and_failures_for_all_tests() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/tests.ocelot",
            "test \"first\" { println(\"one\"); } \
             test \"broken\" { println(missing_value); } \
             test \"third\" { println(\"three\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        let summary = engine.run_tests("examples/tests.ocelot").unwrap();

        assert_eq!(summary.passed, vec!["first", "third"]);
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].name, "broken");
        assert!(summary.failed[0].message.contains("test `broken` failed"));
        assert!(
            summary.failed[0]
                .message
                .contains("unresolved identifier `missing_value`")
        );
        assert_eq!(pal.take_printed_output(), "one\nthree\n");
    }
}
