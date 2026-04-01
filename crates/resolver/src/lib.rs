//! Name resolution for `ocelot`.

use ocelot_ast::script::Script;
use ocelot_base::result::OcelotResult;

/// Resolves names within a parsed program.
pub fn resolve(_script: &Script) -> OcelotResult<()> {
    Ok(())
}
