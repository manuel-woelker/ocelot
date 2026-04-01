//! High-level language pipeline orchestration for `ocelot`.

use ocelot_ast::Program;
use ocelot_base::result::OcelotResult;

/// Result of running the current placeholder language pipeline.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EngineOutput {
    /// The parsed program placeholder.
    pub program: Program,
}

/// Runs the current placeholder pipeline for a source file.
pub fn run(source: &str) -> OcelotResult<EngineOutput> {
    let program = ocelot_parser::parse::parse(source)?;
    ocelot_resolver::resolve(&program)?;
    ocelot_interpreter::interpret(&program)?;
    Ok(EngineOutput { program })
}
