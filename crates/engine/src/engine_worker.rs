use crate::core_module::{CORE_MODULE_NAME, CORE_MODULE_PATH, load_core_module};
use crate::failed_test_result::FailedTestResult;
use crate::loaded_module::LoadedModule;
use crate::loaded_program::LoadedProgram;
use crate::source_file_kind::SourceFileKind;
use crate::test_run_summary::TestRunSummary;
use ocelot_ast::item_kind::ItemKind;
use ocelot_base::assertion_error::render_assertion_error;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::error::{ErrorKind, OcelotError};
use ocelot_base::file_path::FilePath;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::{OcelotResult, OptionExt, ResultExt};
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
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

#[allow(dead_code)]
pub struct EngineWorker {
    pal: PalHandle,
    compilation_context: CompilationContext,
}

impl EngineWorker {
    pub fn new(pal_handle: impl Into<PalHandle>) -> Self {
        Self {
            pal: pal_handle.into(),
            compilation_context: CompilationContext::default(),
        }
    }

    pub fn run_file(self, path: impl Into<FilePath>) -> OcelotResult<()> {
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
        }?;
        Ok(())
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

        let mut modules = vec![load_core_module()?];
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
            crate::engine::validate_loaded_module(module, &mut compilation_context);
            EngineWorker::validate_reserved_core_module_name(module, &mut compilation_context);
        }
        ocelot_resolver::resolution::finish_resolution(&compilation_context)?;

        let compilation_session = self.create_compilation_session();
        let mut symbol_table = self.create_symbol_table();
        let mut module_environments: HashMap<FilePath, ModuleEnvironment> = modules
            .iter()
            .map(|module| (module.source_file.path.clone(), ModuleEnvironment::new()))
            .collect();

        for module in &modules {
            symbol_table.add_module(module.module_name.clone());
        }

        for module in &mut modules {
            ocelot_resolver::resolution::register_module_effects(
                &mut module.script,
                &module.source_file,
                &mut compilation_context,
                &mut symbol_table,
            )?;
        }

        for module in &mut modules {
            ocelot_resolver::resolution::register_module_functions(
                &mut module.script,
                module.module_name.as_str(),
                &module.source_file,
                &mut compilation_context,
                &mut symbol_table,
                module_environments
                    .get_mut(&module.source_file.path)
                    .expect("module environment should exist for loaded module"),
                &compilation_session,
            )?;
        }

        for module in &mut modules {
            ocelot_resolver::resolution::register_module_imports(
                &mut module.script,
                module.module_name.as_str(),
                &module.source_file,
                &mut compilation_context,
                &mut symbol_table,
                module_environments
                    .get_mut(&module.source_file.path)
                    .expect("module environment should exist for loaded module"),
            )?;
        }

        for module in &mut modules {
            ocelot_resolver::resolution::resolve_module_items(
                &mut module.script,
                module.module_name.as_str(),
                &module.source_file,
                &mut compilation_context,
                &symbol_table,
                module_environments
                    .get_mut(&module.source_file.path)
                    .expect("module environment should exist for loaded module"),
                &compilation_session,
            )?;
        }

        let resolved_functions =
            ocelot_resolver::resolution::resolve_user_defined_function_definitions(
                &mut compilation_context,
                &symbol_table,
                &module_environments,
                &compilation_session,
            )?;
        let mut environment = ProgramEnvironment::from_symbol_table(&symbol_table);
        environment.apply_resolved_functions(resolved_functions)?;
        ocelot_resolver::resolution::finish_resolution(&compilation_context)?;

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
        let script = ocelot_parser::parse_script::parse_script(
            &source_file,
            &mut compilation_context.source_diagnostics,
        )?;
        Ok(LoadedModule::new(
            crate::engine::module_name_from_path(execution_root, &path)?,
            kind,
            source_file,
            script,
        ))
    }

    fn load_source_file(&self, path: FilePath) -> OcelotResult<SourceFile> {
        let source = self.pal.read_file_to_string(&path)?;
        Ok(SourceFile::new(path, source))
    }

    fn create_symbol_table(&self) -> SymbolTable {
        SymbolTable::new()
    }

    fn create_compilation_session(&self) -> CompilationSession {
        CompilationSession::with_default_native_functions()
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
}
