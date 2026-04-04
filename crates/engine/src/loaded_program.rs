use crate::loaded_module::LoadedModule;
use ocelot_ast::program_environment::ProgramEnvironment;

/// Parsed multi-file program with one designated entry module.
#[derive(Debug)]
pub struct LoadedProgram {
    pub entry_module_index: usize,
    pub modules: Vec<LoadedModule>,
    pub environment: ProgramEnvironment,
}

impl LoadedProgram {
    /// Creates a loaded program from its modules and shared environment.
    pub fn new(
        entry_module_index: usize,
        modules: Vec<LoadedModule>,
        environment: ProgramEnvironment,
    ) -> Self {
        Self {
            entry_module_index,
            modules,
            environment,
        }
    }

    /// Returns the entry module.
    pub fn entry_module(&self) -> &LoadedModule {
        &self.modules[self.entry_module_index]
    }
}
