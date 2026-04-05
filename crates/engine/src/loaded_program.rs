use crate::loaded_module::ParsedModule;
use ocelot_semantic::program_environment::ProgramEnvironment;

/// Parsed multi-file program with one designated entry module.
#[derive(Debug)]
pub struct LoadedProgram {
    pub entry_module_index: usize,
    pub modules: Vec<ParsedModule>,
    pub environment: ProgramEnvironment,
}

impl LoadedProgram {
    /// Creates a loaded program from its modules and shared environment.
    pub fn new(
        entry_module_index: usize,
        modules: Vec<ParsedModule>,
        environment: ProgramEnvironment,
    ) -> Self {
        Self {
            entry_module_index,
            modules,
            environment,
        }
    }

    /// Returns the entry module.
    pub fn entry_module(&self) -> &ParsedModule {
        &self.modules[self.entry_module_index]
    }
}
