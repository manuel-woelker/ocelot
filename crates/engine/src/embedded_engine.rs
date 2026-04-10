use crate::builtin_module::BuiltinModule;
use crate::compile_program::compile_program;
use crate::core_module::default_core_module;
use ocelot_ast::expression_statement::ExpressionStatement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_base::error::{ErrorKind, OcelotError};
use ocelot_base::file_path::FilePath;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::{OcelotResult, OptionExt};
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_base::source_file::SourceFile;
use ocelot_pal::pal::PalHandle;
use ocelot_semantic::runtime_value::RuntimeValue;

/// Persistent embedding-oriented execution session rooted at one base path.
#[derive(Debug, Clone)]
pub struct EmbeddedEngine {
    pal: PalHandle,
    base_path: FilePath,
    builtin_modules: Vec<BuiltinModule>,
    embedded_source: String,
    source_diagnostics: SourceDiagnostics,
}

impl EmbeddedEngine {
    /// Creates one embedded execution session rooted at `base_path`.
    pub fn new(pal: PalHandle, base_path: impl Into<FilePath>) -> Self {
        Self {
            pal,
            base_path: base_path.into(),
            builtin_modules: vec![default_core_module()],
            embedded_source: String::new(),
            source_diagnostics: SourceDiagnostics::default(),
        }
    }

    /// Registers or replaces one builtin module for this embedded session.
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

    /// Returns the latest compilation diagnostics collected by this session.
    pub fn source_diagnostics(&self) -> &SourceDiagnostics {
        &self.source_diagnostics
    }

    /// Executes one source snippet against the persistent embedded session.
    pub fn execute(&mut self, source: &str) -> OcelotResult<()> {
        let (candidate_source, new_item_offset) = self.candidate_source(source);
        let synthetic_source_file = self.synthetic_source_file(candidate_source);
        let resolved_program = self.compile_with_source(synthetic_source_file)?;
        let entry_module = resolved_program
            .modules
            .iter()
            .find(|module| module.source_file.path == self.synthetic_entry_path())
            .context("internal error: embedded entry module was not loaded")?;
        let statements = entry_module
            .compilation_unit
            .statements()
            .filter(|statement| statement.span.start() >= new_item_offset)
            .cloned()
            .collect::<Vec<_>>();

        ocelot_interpreter::interpreter::Interpreter::new(
            &*self.pal,
            &entry_module.source_file,
            &resolved_program.symbol_table,
        )
        .interpret_statements(&statements)?;

        self.embedded_source = entry_module.source_file.source().to_owned();
        Ok(())
    }

    /// Evaluates one expression against the persistent embedded session.
    pub fn eval(&mut self, expression_source: &str) -> OcelotResult<RuntimeValue> {
        let eval_statement = format!("{expression_source};");
        let (candidate_source, new_item_offset) = self.candidate_source(eval_statement.as_str());
        let synthetic_source_file = self.synthetic_source_file(candidate_source);
        let resolved_program = self.compile_with_source(synthetic_source_file)?;
        let entry_module = resolved_program
            .modules
            .iter()
            .find(|module| module.source_file.path == self.synthetic_entry_path())
            .context("internal error: embedded entry module was not loaded")?;
        let statement = entry_module
            .compilation_unit
            .statements()
            .find(|statement| statement.span.start() >= new_item_offset)
            .context("internal error: eval did not produce a statement")?;
        let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;

        ocelot_interpreter::interpreter::Interpreter::new(
            &*self.pal,
            &entry_module.source_file,
            &resolved_program.symbol_table,
        )
        .evaluate_expression(expression)
    }

    fn candidate_source(&self, snippet: &str) -> (String, usize) {
        if self.embedded_source.is_empty() {
            return (snippet.to_owned(), 0);
        }

        let offset = self.embedded_source.len() + 1;
        (format!("{}\n{}", self.embedded_source, snippet), offset)
    }

    fn compile_with_source(
        &mut self,
        synthetic_source_file: SourceFile,
    ) -> OcelotResult<ocelot_semantic::resolved_program::ResolvedProgram> {
        let result = compile_program(
            &*self.pal,
            &self.base_path,
            &self.synthetic_entry_path(),
            &self.builtin_modules,
            &[synthetic_source_file],
            &mut self.source_diagnostics,
        );

        result.map_err(|error| attach_compilation_diagnostics(error, &self.source_diagnostics))
    }

    fn synthetic_entry_path(&self) -> FilePath {
        self.base_path.join("__embedded_main.ocelot-script")
    }

    fn synthetic_source_file(&self, source: impl Into<SharedString>) -> SourceFile {
        SourceFile::new(self.synthetic_entry_path(), source)
    }
}

fn attach_compilation_diagnostics(
    error: OcelotError,
    source_diagnostics: &SourceDiagnostics,
) -> OcelotError {
    let ErrorKind::CompilationError(_) = error.kind() else {
        return error;
    };

    if source_diagnostics.diagnostics.is_empty() {
        return error;
    }

    error.with_source(OcelotError::message(render_source_diagnostics(
        &source_diagnostics.diagnostics,
    )))
}

#[cfg(test)]
mod tests {
    use super::EmbeddedEngine;
    use ocelot_base::error::ErrorKind;
    use ocelot_base::render_source_diagnostics::render_source_diagnostics;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;
    use ocelot_semantic::runtime_value::RuntimeValue;

    #[test]
    fn eval_returns_runtime_values_without_disk_backed_embedded_files() {
        let pal = PalMock::new();
        let mut engine = EmbeddedEngine::new(PalHandle::new(pal), "examples");

        let result = engine.eval("\"hello\"").unwrap();

        assert_eq!(result, RuntimeValue::string("hello"));
    }

    #[test]
    fn execute_retains_function_definitions_for_later_eval_calls() {
        let pal = PalMock::new();
        let mut engine = EmbeddedEngine::new(PalHandle::new(pal), "examples");

        engine.execute("fun greet() { println(\"hi\"); }").unwrap();
        let result = engine.eval("greet()").unwrap();

        assert_eq!(result, RuntimeValue::unit());
    }

    #[test]
    fn execute_does_not_replay_earlier_top_level_statements() {
        let pal = PalMock::new();
        let mut engine = EmbeddedEngine::new(PalHandle::new(pal.clone()), "examples");

        engine.execute("println(\"first\");").unwrap();
        engine.execute("println(\"second\");").unwrap();

        assert_eq!(pal.take_printed_output(), "first\nsecond\n");
    }

    #[test]
    fn eval_can_load_modules_from_the_configured_base_path() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/helper.ocelot",
            "fun greet() { core::println(\"from module\"); }",
        );
        let mut engine = EmbeddedEngine::new(PalHandle::new(pal.clone()), "examples");

        let result = engine.eval("helper::greet()").unwrap();

        assert_eq!(result, RuntimeValue::unit());
        assert_eq!(pal.take_printed_output(), "from module\n");
    }

    #[test]
    fn failed_execute_keeps_the_previous_valid_session_state() {
        let pal = PalMock::new();
        let mut engine = EmbeddedEngine::new(PalHandle::new(pal), "examples");

        engine.execute("fun greet() { println(\"hi\"); }").unwrap();
        let error = engine.execute("fun broken( {").unwrap_err();
        let result = engine.eval("greet()").unwrap();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(ocelot_base::compilation_stage::CompilationStage::Parser)
        ));
        assert_eq!(result, RuntimeValue::unit());
    }

    #[test]
    fn eval_reports_parser_diagnostics() {
        let pal = PalMock::new();
        let mut engine = EmbeddedEngine::new(PalHandle::new(pal), "examples");

        let error = engine.eval("not").unwrap_err();

        assert!(
            render_source_diagnostics(&engine.source_diagnostics().diagnostics)
                .contains("expected expression")
        );
        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(ocelot_base::compilation_stage::CompilationStage::Parser)
        ));
    }

    #[test]
    fn eval_reports_resolver_diagnostics() {
        let pal = PalMock::new();
        let mut engine = EmbeddedEngine::new(PalHandle::new(pal), "examples");

        let error = engine.eval("helper::missing()").unwrap_err();

        assert!(
            render_source_diagnostics(&engine.source_diagnostics().diagnostics)
                .contains("unknown module `helper`")
        );
        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(ocelot_base::compilation_stage::CompilationStage::Resolver)
        ));
    }
}
