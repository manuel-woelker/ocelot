use crate::parsed_module::ParsedModule;
use crate::symbol_table::SymbolTable;
use ocelot_base::file_path::FilePath;
use ocelot_base::source_diagnostics::SourceDiagnostics;

/// Fully resolved multi-module program state returned by the resolver.
#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub entry_path: FilePath,
    pub modules: Vec<ParsedModule>,
    pub source_diagnostics: SourceDiagnostics,
    pub symbol_table: SymbolTable,
}

impl ResolvedProgram {
    /// Creates one resolved multi-module program from its modules and semantic outputs.
    pub fn new(
        entry_path: FilePath,
        modules: Vec<ParsedModule>,
        source_diagnostics: SourceDiagnostics,
        symbol_table: SymbolTable,
    ) -> Self {
        Self {
            entry_path,
            modules,
            source_diagnostics,
            symbol_table,
        }
    }
}
