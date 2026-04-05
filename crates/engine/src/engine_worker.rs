use crate::builtin_module::BuiltinModule;
use crate::engine_command::{EngineCommand, RunCommandKind};
use crate::failed_test_result::FailedTestResult;
use crate::loaded_module::ParsedModule;
use crate::module_name_from_path::module_name_from_path;
use crate::source_file_kind::SourceFileKind;
use crate::test_run_summary::TestRunSummary;
use ocelot_ast::item_kind::ItemKind;
use ocelot_base::assertion_error::render_assertion_error;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::error::{ErrorKind, OcelotError};
use ocelot_base::file_path::FilePath;
use ocelot_base::line_bounds::LineBounds;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::{OcelotResult, OptionExt, ResultExt};
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_pal::pal::PalHandle;
use ocelot_semantic::compilation_context::CompilationContext;
use ocelot_semantic::compilation_session::CompilationSession;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::module_environment::ModuleEnvironment;
use ocelot_semantic::program_environment::ProgramEnvironment;
use ocelot_semantic::symbol_table::SymbolTable;
use std::collections::HashMap;

pub struct EngineWorker {
    pal: PalHandle,
    compilation_context: CompilationContext,
    command: EngineCommand,
    builtin_modules: Vec<BuiltinModule>,
    parsed_modules: Vec<ParsedModule>,
    program_environment: ProgramEnvironment,
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
            compilation_context: CompilationContext::default(),
            command,
            builtin_modules,
            parsed_modules: Vec::new(),
            program_environment: ProgramEnvironment::default(),
            test_run_summary: TestRunSummary::default(),
        }
    }

    pub fn run_command(&mut self) -> OcelotResult<()> {
        self.parse_modules()?;
        self.early_abort(CompilationStage::Parser)?;
        self.resolve_modules()?;
        self.early_abort(CompilationStage::Resolver)?;
        self.execute()
    }

    pub fn source_diagnostics(&self) -> &SourceDiagnostics {
        &self.compilation_context.source_diagnostics
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
        }
    }

    fn execute_entry_module(&self) -> OcelotResult<()> {
        let entry_module = self.entry_module()?;

        match entry_module.kind {
            SourceFileKind::Script => ocelot_interpreter::interpret_script::interpret_script(
                &entry_module.compilation_unit,
                &entry_module.source_file,
                &self.program_environment,
                &*self.pal,
            ),
            SourceFileKind::Module => {
                self.run_module_entrypoint(entry_module, &self.program_environment)
            }
        }
    }

    fn run_selected_test(&self, test_name: &str) -> OcelotResult<()> {
        let entry_module = self.entry_module()?;
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
            &self.program_environment,
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
        let entry_module = self.entry_module()?;
        let interpreter = ocelot_interpreter::interpreter::Interpreter::new(
            &*self.pal,
            &entry_module.source_file,
            &self.program_environment,
        );
        let mut summary = TestRunSummary::new();

        for item in &entry_module.compilation_unit.items {
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

    fn resolve_modules(&mut self) -> OcelotResult<()> {
        for module in &self.parsed_modules {
            validate_loaded_module(module, &mut self.compilation_context);
            validate_builtin_module_conflict(
                module,
                &self.builtin_modules,
                &mut self.compilation_context,
            );
        }

        let compilation_session = self.create_compilation_session();
        let mut symbol_table = self.create_symbol_table();
        let mut module_environments: HashMap<FilePath, ModuleEnvironment> = self
            .parsed_modules
            .iter()
            .map(|module| (module.source_file.path.clone(), ModuleEnvironment::new()))
            .collect();

        for module in &self.parsed_modules {
            symbol_table.add_module(module.module_name.clone());
        }

        for module in &mut self.parsed_modules {
            ocelot_resolver::resolution::register_module_effects(
                &mut module.compilation_unit,
                &module.source_file,
                &mut self.compilation_context,
                &mut symbol_table,
            )?;
        }

        for module in &mut self.parsed_modules {
            ocelot_resolver::resolution::register_module_functions(
                &mut module.compilation_unit,
                module.module_name.as_str(),
                &module.source_file,
                &mut self.compilation_context,
                &mut symbol_table,
                module_environments
                    .get_mut(&module.source_file.path)
                    .expect("module environment should exist for loaded module"),
                &compilation_session,
            )?;
        }

        for module in &mut self.parsed_modules {
            ocelot_resolver::resolution::register_module_imports(
                &mut module.compilation_unit,
                module.module_name.as_str(),
                &module.source_file,
                &mut self.compilation_context,
                &mut symbol_table,
                module_environments
                    .get_mut(&module.source_file.path)
                    .expect("module environment should exist for loaded module"),
            )?;
        }

        for module in &mut self.parsed_modules {
            ocelot_resolver::resolution::resolve_module_items(
                &mut module.compilation_unit,
                module.module_name.as_str(),
                &module.source_file,
                &mut self.compilation_context,
                &symbol_table,
                module_environments
                    .get(&module.source_file.path)
                    .expect("module environment should exist for loaded module"),
                &compilation_session,
            )?;
        }

        let resolved_functions =
            ocelot_resolver::resolution::resolve_user_defined_function_definitions(
                &mut self.compilation_context,
                &symbol_table,
                &module_environments,
                &compilation_session,
            )?;
        self.program_environment = ProgramEnvironment::from_symbol_table(&symbol_table);
        self.program_environment
            .apply_resolved_functions(resolved_functions)?;

        Ok(())
    }

    fn early_abort(&self, stage: CompilationStage) -> OcelotResult<()> {
        if self.compilation_context.has_errors() {
            return Err(OcelotError::compilation_error(stage));
        }
        Ok(())
    }

    fn parse_modules(&mut self) -> OcelotResult<()> {
        self.parsed_modules.clear();

        for file in self.collect_files()? {
            if let Some(module) = self.parse_module(&file)? {
                self.parsed_modules.push(module);
            }
        }

        let builtin_modules = self.builtin_modules.clone();
        for builtin_module in builtin_modules {
            if let Some(module) = self.parse_builtin_module(builtin_module)? {
                self.parsed_modules.push(module);
            }
        }
        Ok(())
    }

    fn collect_files(&self) -> OcelotResult<Vec<FilePath>> {
        let mut file_paths = self
            .pal
            .walk_directory(&self.command.base_path, &[String::from("*.ocelot")])?
            .collect::<OcelotResult<Vec<_>>>()?;
        let entry_path = self.command.entry_path();
        if !file_paths.contains(entry_path) {
            file_paths.push(entry_path.clone());
        }
        file_paths.sort();
        Ok(file_paths)
    }

    fn parse_module(&mut self, path: &FilePath) -> OcelotResult<Option<ParsedModule>> {
        let source_file = self.read_source_file(path.clone())?;
        let base_path = self.command.base_path.clone();
        let path = path.clone();
        self.parse_source_file(source_file, |source_file, compilation_unit| {
            let source_kind = SourceFileKind::from_path(&path)?;
            Ok(ParsedModule::new(
                module_name_from_path(&base_path, &path)?,
                source_kind,
                source_file,
                compilation_unit,
            ))
        })
    }

    fn parse_builtin_module(
        &mut self,
        builtin_module: BuiltinModule,
    ) -> OcelotResult<Option<ParsedModule>> {
        let source_file = SourceFile::new(builtin_module.source_file_path(), builtin_module.source);
        self.parse_source_file(source_file, |source_file, compilation_unit| {
            Ok(ParsedModule::new(
                builtin_module.module_name,
                SourceFileKind::Module,
                source_file,
                compilation_unit,
            ))
        })
    }

    fn parse_source_file(
        &mut self,
        source_file: SourceFile,
        build_module: impl FnOnce(
            SourceFile,
            ocelot_ast::compilation_unit::CompilationUnit,
        ) -> OcelotResult<ParsedModule>,
    ) -> OcelotResult<Option<ParsedModule>> {
        let compilation_unit = match ocelot_parser::parse_compilation_unit::parse_compilation_unit(
            &source_file,
            &mut self.compilation_context.source_diagnostics,
        ) {
            Ok(compilation_unit) => compilation_unit,
            Err(error) if is_parser_compilation_error(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        Ok(Some(build_module(source_file, compilation_unit)?))
    }

    fn read_source_file(&self, path: FilePath) -> OcelotResult<SourceFile> {
        let source = self.pal.read_file_to_string(&path)?;
        Ok(SourceFile::new(path, source))
    }

    fn entry_module(&self) -> OcelotResult<&ParsedModule> {
        self.parsed_modules
            .iter()
            .find(|module| module.source_file.path == *self.command.entry_path())
            .context("internal error: entry module was not loaded")
    }

    fn create_symbol_table(&self) -> SymbolTable {
        SymbolTable::new()
    }

    fn create_compilation_session(&self) -> CompilationSession {
        CompilationSession::with_default_native_functions()
    }

    fn run_module_entrypoint(
        &self,
        entry_module: &ParsedModule,
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

fn is_parser_compilation_error(error: &OcelotError) -> bool {
    matches!(
        error.kind(),
        ErrorKind::CompilationError(CompilationStage::Parser)
    )
}

fn validate_builtin_module_conflict(
    module: &ParsedModule,
    builtin_modules: &[BuiltinModule],
    compilation_context: &mut CompilationContext,
) {
    let Some(builtin_module) = builtin_modules
        .iter()
        .find(|builtin_module| module.module_name == builtin_module.module_name)
    else {
        return;
    };

    if module.source_file.path == builtin_module.source_file_path() {
        return;
    }

    compilation_context.add_diagnostic(
        SourceDiagnostic::new(
            DiagnosticLevel::Error,
            &module.source_file.path,
            format!(
                "module name `{}` is reserved for a builtin module",
                module.module_name
            ),
        )
        .with_excerpt(source_excerpt_for_path(
            &module.source_file,
            "rename this file or module path",
        )),
    );

    fn source_excerpt_for_path(
        source_file: &SourceFile,
        annotation: impl Into<SharedString>,
    ) -> SourceExcerpt {
        let annotation = annotation.into();
        let source_line = source_file.source().lines().next().unwrap_or_default();

        SourceExcerpt::new(&source_file.path, 1, source_line)
            .with_annotation(SourceAnnotation::new(Span::new(0, 0), annotation))
    }
}

fn validate_loaded_module(module: &ParsedModule, compilation_context: &mut CompilationContext) {
    if module.kind.allows_top_level_statements() {
        return;
    }

    let Some(statement) = module.compilation_unit.statements().next() else {
        return;
    };

    compilation_context.add_diagnostic(module_statement_diagnostic(
        &module.source_file,
        statement.span.clone(),
    ));
}

fn module_statement_diagnostic(source_file: &SourceFile, span: Span) -> SourceDiagnostic {
    let line_bounds = LineBounds::new(source_file.source(), span.start());
    let source_line = &source_file.source()[line_bounds.line_start..line_bounds.line_end];
    let relative_start = span.start().saturating_sub(line_bounds.line_start);
    let relative_end = span.end().saturating_sub(line_bounds.line_start);

    SourceDiagnostic::new(
        DiagnosticLevel::Error,
        &source_file.path,
        "top-level statements are only allowed in `.ocelot-script` files",
    )
    .with_excerpt(
        SourceExcerpt::new(&source_file.path, line_bounds.line_number, source_line)
            .with_annotation(SourceAnnotation::new(
                Span::new(relative_start, relative_end),
                "move this statement into `main()` or rename the file to `.ocelot-script`",
            )),
    )
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
