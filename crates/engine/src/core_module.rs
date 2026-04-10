use crate::builtin_module::BuiltinModule;

pub const CORE_MODULE_NAME: &str = "core";
const CORE_MODULE_SOURCE: &str = include_str!("../resources/builtin/core.ocelot");

pub fn default_core_module() -> BuiltinModule {
    BuiltinModule::new(CORE_MODULE_NAME, CORE_MODULE_SOURCE)
}
