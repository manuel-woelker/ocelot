use crate::discovered_test::DiscoveredTest;
use crate::failed_test_result::FailedTestResult;
use crate::loaded_module::LoadedModule;
use crate::loaded_program::LoadedProgram;
use crate::test_run_summary::TestRunSummary;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::program_environment::ProgramEnvironment;
use ocelot_base::assertion_error::render_assertion_error;
use ocelot_base::error::ErrorKind;
use ocelot_base::error::OcelotError;
use ocelot_base::file_path::FilePath;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::OcelotResult;
use ocelot_base::result::OptionExt;
use ocelot_base::result::ResultExt;
use ocelot_base::shared_string::SharedString;
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
        let program = self.compile_program(path.into())?;
        let entry_module = program.entry_module();
        ocelot_interpreter::interpret_script::interpret_script(
            &entry_module.script,
            &entry_module.source_file,
            &program.environment,
            &*self.pal,
        )?;
        Ok(())
    }

    pub fn discover_tests(&self, path: impl Into<FilePath>) -> OcelotResult<Vec<DiscoveredTest>> {
        let program = self.compile_program(path.into())?;
        let entry_module = program.entry_module();

        Ok(entry_module
            .script
            .items
            .iter()
            .filter_map(|item| match &item.kind {
                ItemKind::Function(_) => None,
                ItemKind::Test(test_item) => Some(DiscoveredTest::new(
                    test_item.name.clone(),
                    test_item.span.clone(),
                )),
                ItemKind::Statement(_) => None,
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

        let entry_module_index = module_paths
            .iter()
            .position(|path| path == &entry_path)
            .context("internal error: entry module was not loaded")?;
        let mut modules = module_paths
            .into_iter()
            .map(|path| self.load_module(&execution_root, path))
            .collect::<OcelotResult<Vec<_>>>()?;

        let mut compilation_context =
            ocelot_base::compilation_context::CompilationContext::default();
        let mut environment = self.create_program_environment();

        for module in &modules {
            environment.add_module(module.module_name.clone());
        }

        for module in &mut modules {
            ocelot_resolver::register_module_functions(
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
            )?;
        }

        ocelot_resolver::resolve_user_defined_function_definitions(
            &mut compilation_context,
            &mut environment,
        )?;
        ocelot_resolver::finish_resolution(&compilation_context)?;

        Ok(LoadedProgram::new(entry_module_index, modules, environment))
    }

    fn load_module(&self, execution_root: &FilePath, path: FilePath) -> OcelotResult<LoadedModule> {
        let source_file = self.load_source_file(path.clone())?;
        let mut compilation_context =
            ocelot_base::compilation_context::CompilationContext::default();
        let script =
            ocelot_parser::parse_script::parse_script(&source_file, &mut compilation_context)?;
        Ok(LoadedModule::new(
            module_name_from_path(execution_root, &path)?,
            source_file,
            script,
        ))
    }

    fn load_source_file(&self, path: FilePath) -> OcelotResult<SourceFile> {
        let source = self.pal.read_file_to_string(&path)?;
        Ok(SourceFile::new(path, source))
    }

    fn create_program_environment(&self) -> ProgramEnvironment {
        ProgramEnvironment::new()
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

#[cfg(test)]
mod tests {
    use super::Engine;
    use super::module_name_from_path;
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
    fn run_script_executes_functions_from_sibling_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot", "helper::greet();");
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet() { println(\"hello\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_script("examples/main.ocelot").unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn run_script_executes_functions_from_nested_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot", "math::greet::hello();");
        pal.set_file(
            "examples/math/greet.ocelot",
            "fun hello() { println(\"hello\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_script("examples/main.ocelot").unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn run_script_does_not_execute_top_level_statements_from_non_entry_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot", "helper::greet();");
        pal.set_file(
            "examples/helper.ocelot",
            "println(\"setup\"); fun greet() { println(\"hello\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_script("examples/main.ocelot").unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn run_script_reports_unknown_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot", "helper::greet();");

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine.run_script("examples/main.ocelot").unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(error.to_test_string().contains("unknown module `helper`"));
    }

    #[test]
    fn run_script_reports_missing_functions_in_loaded_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot", "helper::greet();");
        pal.set_file("examples/helper.ocelot", "fun wave() {}");

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine.run_script("examples/main.ocelot").unwrap_err();

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
    fn run_test_executes_only_entry_module_tests() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot", "test \"main\" { helper::greet(); }");
        pal.set_file(
            "examples/helper.ocelot",
            "test \"helper\" { println(\"wrong\"); } fun greet() { println(\"hello\"); }",
        );

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_test("examples/main.ocelot", "main").unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn run_test_reports_cross_file_runtime_diagnostics_with_the_called_file_path() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/main.ocelot",
            "test \"broken\" { helper::greet(); }",
        );
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet() { println(missing_value); }",
        );

        let engine = Engine::new(PalHandle::new(pal));

        let error = engine
            .run_test("examples/main.ocelot", "broken")
            .unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("unresolved identifier `missing_value`")
        );
        assert!(error.to_test_string().contains("examples/helper.ocelot"));
    }
}
