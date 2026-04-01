//! High-level language pipeline orchestration for `ocelot`.

use ocelot_ast::script::Script;
use ocelot_base::result::OcelotResult;
use ocelot_base::source_file::SourceFile;

/// Result of running the current placeholder language pipeline.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EngineOutput {
    /// The parsed script.
    pub script: Script,
}

/// Runs the current placeholder pipeline for a source file.
pub fn run(source: &str) -> OcelotResult<EngineOutput> {
    let source_file = SourceFile::new("<anonymous>", source);
    let script = ocelot_parser::parse_script::parse_script(&source_file)?;
    ocelot_resolver::resolve(&script)?;
    ocelot_interpreter::interpret(&script)?;
    Ok(EngineOutput { script })
}
