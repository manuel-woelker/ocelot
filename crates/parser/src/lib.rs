//! Parsing support for `ocelot`.

use ocelot_ast::Program;
use ocelot_base::result::OcelotResult;

/// Parses source text into an AST.
pub fn parse(_source: &str) -> OcelotResult<Program> {
    Ok(Program)
}
