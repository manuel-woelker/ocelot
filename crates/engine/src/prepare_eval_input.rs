use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::error::OcelotError;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::OcelotResult;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_base::source_file::SourceFile;

/// Validates the shared embedded-eval input contract without running code.
pub fn prepare_eval_input(
    file: &SourceFile,
    diagnostics: &mut SourceDiagnostics,
) -> OcelotResult<()> {
    let input = ocelot_parser::parse_compilation_unit::parse_compilation_unit(file, diagnostics)?;
    if input.items.len() == 1
        && matches!(&input.items[0].kind,
        ItemKind::Statement(statement) if matches!(statement.kind, StatementKind::Expression(_)))
    {
        return Ok(());
    }
    diagnostics.diagnostics.push(SourceDiagnostic::new(
        DiagnosticLevel::Error,
        &file.path,
        "eval expects exactly one expression",
    ));
    Err(
        OcelotError::compilation_error(CompilationStage::Parser).with_source(OcelotError::message(
            render_source_diagnostics(&diagnostics.diagnostics),
        )),
    )
}
