//! Tree-walking interpreter support for `ocelot`.

use ocelot_ast::script::Script;
use ocelot_base::result::OcelotResult;

/// Executes a parsed program.
pub fn interpret(_script: &Script) -> OcelotResult<()> {
    Ok(())
}
