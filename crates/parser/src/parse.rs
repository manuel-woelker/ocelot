use ocelot_ast::script::Script;
use ocelot_base::result::OcelotResult;

/// Parses source text into an AST.
pub fn parse(_source: &str) -> OcelotResult<Script> {
    Ok(Script::default())
}
