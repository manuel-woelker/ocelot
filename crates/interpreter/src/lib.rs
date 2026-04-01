//! Tree-walking interpreter support for `ocelot`.

use ocelot_ast::Program;
use ocelot_base::result::OcelotResult;

/// Executes a parsed program.
pub fn interpret(_program: &Program) -> OcelotResult<()> {
    Ok(())
}
