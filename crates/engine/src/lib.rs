//! High-level language pipeline orchestration for `ocelot`.

use ocelot_ast::script::Script;
use ocelot_base::result::OcelotResult;

/// Result of running the current placeholder language pipeline.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EngineOutput {
    /// The parsed script.
    pub script: Script,
}

/// Runs the current placeholder pipeline for a source file.
pub fn run(source: &str) -> OcelotResult<EngineOutput> {
    let script = ocelot_parser::parse::parse(source)?;
    ocelot_resolver::resolve(&script)?;
    ocelot_interpreter::interpret(&script)?;
    Ok(EngineOutput { script })
}
