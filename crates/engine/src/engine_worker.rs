use crate::builtin_module::BuiltinModule;
use crate::compile_program::compile_program;
use crate::engine_command::{EngineCommand, RunCommandKind};
use crate::failed_test_result::FailedTestResult;
use crate::passed_test_result::PassedTestResult;
use crate::test_run_summary::TestRunSummary;
use ocelot_ast::item_kind::ItemKind;
use ocelot_base::assertion_error::render_assertion_error;
use ocelot_base::error::{ErrorKind, OcelotError};
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::{OcelotResult, OptionExt, ResultExt};
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_pal::pal::PalHandle;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::parsed_module::ParsedModule;
use ocelot_semantic::resolved_program::ResolvedProgram;
use ocelot_semantic::source_file_kind::SourceFileKind;
use ocelot_semantic::symbol_table::SymbolTable;

pub struct EngineWorker {
    pal: PalHandle,
    source_diagnostics: SourceDiagnostics,
    command: EngineCommand,
    builtin_modules: Vec<BuiltinModule>,
    resolved_program: Option<ResolvedProgram>,
    test_run_summary: TestRunSummary,
}

impl EngineWorker {
    pub fn new(
        pal_handle: impl Into<PalHandle>,
        command: EngineCommand,
        builtin_modules: Vec<BuiltinModule>,
    ) -> Self {
        Self {
            pal: pal_handle.into(),
            source_diagnostics: SourceDiagnostics::default(),
            command,
            builtin_modules,
            resolved_program: None,
            test_run_summary: TestRunSummary::default(),
        }
    }

    pub fn run_command(&mut self) -> OcelotResult<()> {
        self.resolved_program = None;
        let resolved_program = compile_program(
            &*self.pal,
            &self.command.base_path,
            self.command.entry_path(),
            &self.builtin_modules,
            &[],
            &mut self.source_diagnostics,
        )?;
        self.resolved_program = Some(resolved_program);
        self.execute()
    }

    pub fn source_diagnostics(&self) -> &SourceDiagnostics {
        &self.source_diagnostics
    }

    pub fn test_run_summary(&self) -> &TestRunSummary {
        &self.test_run_summary
    }

    fn execute(&mut self) -> OcelotResult<()> {
        match &self.command.kind {
            RunCommandKind::RunFile { .. } => self.execute_entry_module(),
            RunCommandKind::RunTest { test_name, .. } => self.run_selected_test(test_name.as_str()),
            RunCommandKind::RunTests { .. } => {
                self.test_run_summary = self.run_all_tests()?;
                Ok(())
            }
            RunCommandKind::RunNamedTests { test_names, .. } => {
                self.test_run_summary = self.run_named_tests(test_names)?;
                Ok(())
            }
        }
    }

    fn execute_entry_module(&self) -> OcelotResult<()> {
        let entry_module = self.entry_module()?;
        let symbol_table = self.symbol_table();

        match entry_module.kind {
            SourceFileKind::Script => ocelot_interpreter::interpret_script::interpret_script(
                &entry_module.compilation_unit,
                &entry_module.source_file,
                symbol_table,
                &*self.pal,
            ),
            SourceFileKind::Module => self.run_module_entrypoint(entry_module, symbol_table),
        }
    }

    fn run_selected_test(&self, test_name: &str) -> OcelotResult<()> {
        let entry_module = self.entry_module()?;
        let symbol_table = self.symbol_table();
        let test_item = entry_module
            .compilation_unit
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
            symbol_table,
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

    fn run_all_tests(&self) -> OcelotResult<TestRunSummary> {
        self.run_named_tests_for_entry_module(None)
    }

    fn run_named_tests(&self, test_names: &[SharedString]) -> OcelotResult<TestRunSummary> {
        self.run_named_tests_for_entry_module(Some(test_names))
    }

    fn run_named_tests_for_entry_module(
        &self,
        test_names: Option<&[SharedString]>,
    ) -> OcelotResult<TestRunSummary> {
        let entry_module = self.entry_module()?;
        let symbol_table = self.symbol_table();
        let interpreter = ocelot_interpreter::interpreter::Interpreter::new(
            &*self.pal,
            &entry_module.source_file,
            symbol_table,
        );
        let mut summary = TestRunSummary::new();

        for item in &entry_module.compilation_unit.items {
            let ItemKind::Test(test_item) = &item.kind else {
                continue;
            };

            if let Some(test_names) = test_names
                && !test_names
                    .iter()
                    .any(|test_name| test_name == &test_item.name)
            {
                continue;
            }

            match interpreter.interpret_statements(&test_item.body) {
                Ok(()) => summary.passed.push(PassedTestResult::new(
                    entry_module.source_file.path.clone(),
                    test_item.name.clone(),
                )),
                Err(error) => match error.kind() {
                    ErrorKind::AssertionError(assertion_error) => {
                        summary.failed.push(FailedTestResult::new(
                            entry_module.source_file.path.clone(),
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
                            entry_module.source_file.path.clone(),
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
                        entry_module.source_file.path.clone(),
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

    fn entry_module(&self) -> OcelotResult<&ParsedModule> {
        let resolved_program = self
            .resolved_program
            .as_ref()
            .context("internal error: program was not resolved")?;

        resolved_program
            .modules
            .iter()
            .find(|module| module.source_file.path == resolved_program.entry_path)
            .context("internal error: entry module was not loaded")
    }

    fn symbol_table(&self) -> &SymbolTable {
        &self
            .resolved_program
            .as_ref()
            .expect("program should be resolved before execution")
            .symbol_table
    }

    fn run_module_entrypoint(
        &self,
        entry_module: &ParsedModule,
        symbol_table: &SymbolTable,
    ) -> OcelotResult<()> {
        let entrypoint_name = symbol_table.qualify_function_name(&entry_module.module_name, "main");
        let function_index = symbol_table
            .resolve_function_exact(entrypoint_name.as_str())
            .context(format!(
                "module `{}` does not define a `main()` entrypoint",
                entry_module.module_name
            ))?;
        let function_definition = symbol_table.function_definition(function_index)?;
        let FunctionKind::UserDefined {
            function,
            source_file,
        } = &function_definition.kind
        else {
            ocelot_base::bail!(
                "internal error: module entrypoint `{entrypoint_name}` must be user-defined"
            );
        };

        ocelot_interpreter::interpreter::Interpreter::new(&*self.pal, source_file, symbol_table)
            .interpret_statements(&function.body)
    }
}

#[cfg(test)]
mod tests {
    use super::EngineWorker;
    use crate::builtin_module::BuiltinModule;
    use crate::engine_command::EngineCommand;
    use ocelot_base::compilation_stage::CompilationStage;
    use ocelot_base::error::ErrorKind;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;

    #[test]
    fn run_command_collects_parser_diagnostics_in_compilation_context() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "println(\"hello\"");

        let mut worker = EngineWorker::new(
            PalHandle::new(pal),
            EngineCommand::run_file("examples/main.ocelot-script".into()).unwrap(),
            Vec::new(),
        );

        let error = worker.run_command().unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Parser)
        ));
        assert_eq!(worker.source_diagnostics().diagnostics.len(), 1);
        assert_eq!(
            worker.source_diagnostics().diagnostics[0]
                .file_path
                .as_str(),
            "examples/main.ocelot-script"
        );
    }

    #[test]
    fn run_command_collects_resolver_diagnostics_in_compilation_context() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "helper::greet();");
        pal.set_file(
            "examples/helper.ocelot",
            "println(\"setup\"); fun greet() { println(\"hello\"); }",
        );

        let mut worker = EngineWorker::new(
            PalHandle::new(pal),
            EngineCommand::run_file("examples/main.ocelot-script".into()).unwrap(),
            Vec::new(),
        );

        let error = worker.run_command().unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(
            worker
                .source_diagnostics()
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.file_path.as_str() == "examples/helper.ocelot")
        );
    }

    #[test]
    fn run_command_rejects_user_modules_that_conflict_with_builtin_modules() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "helpers::greet();");
        pal.set_file("examples/helpers.ocelot", "fun greet() {}");

        let mut worker = EngineWorker::new(
            PalHandle::new(pal),
            EngineCommand::run_file("examples/main.ocelot-script".into()).unwrap(),
            vec![BuiltinModule::new(
                "helpers",
                "fun greet() { core::println(\"builtin\"); }",
            )],
        );

        let error = worker.run_command().unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(
            worker
                .source_diagnostics()
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message
                    == "module name `helpers` is reserved for a builtin module")
        );
    }
}
