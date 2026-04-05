use crate::parsed_module::ParsedModule;
use crate::program_environment::ProgramEnvironment;
use crate::symbol_table::SymbolTable;
use ocelot_base::source_diagnostics::SourceDiagnostics;

/// Fully resolved multi-module program state returned by the resolver.
#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub modules: Vec<ParsedModule>,
    pub source_diagnostics: SourceDiagnostics,
    pub symbol_table: SymbolTable,
}

impl ResolvedProgram {
    /// Creates one resolved multi-module program from its modules and semantic outputs.
    pub fn new(
        modules: Vec<ParsedModule>,
        source_diagnostics: SourceDiagnostics,
        symbol_table: SymbolTable,
    ) -> Self {
        Self {
            modules,
            source_diagnostics,
            symbol_table,
        }
    }

    /// Builds the interpreter-facing program environment from the semantic symbol table.
    pub fn program_environment(&self) -> ProgramEnvironment {
        ProgramEnvironment::from_symbol_table(&self.symbol_table)
    }
}
