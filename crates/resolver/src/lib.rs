//! Name resolution for `ocelot`.

use ocelot_ast::Program;
use ocelot_base::result::OcelotResult;

/// Resolves names within a parsed program.
pub fn resolve(_program: &Program) -> OcelotResult<()> {
    Ok(())
}
