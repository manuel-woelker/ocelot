use crate::loaded_module::ParsedModule;
use crate::source_file_kind::SourceFileKind;
use ocelot_base::result::OcelotResult;
use ocelot_base::source_file::SourceFile;
use ocelot_semantic::compilation_context::CompilationContext;

pub const CORE_MODULE_NAME: &str = "core";
pub const CORE_MODULE_PATH: &str = "crates/engine/resources/core.ocelot";
const CORE_MODULE_SOURCE: &str = include_str!("../resources/core.ocelot");

pub fn load_core_module() -> OcelotResult<ParsedModule> {
    let source_file = SourceFile::new(CORE_MODULE_PATH, CORE_MODULE_SOURCE);
    let mut compilation_context = CompilationContext::default();
    let script = ocelot_parser::parse_script::parse_script(
        &source_file,
        &mut compilation_context.source_diagnostics,
    )?;
    Ok(ParsedModule::new(
        CORE_MODULE_NAME,
        SourceFileKind::Module,
        source_file,
        script,
    ))
}
