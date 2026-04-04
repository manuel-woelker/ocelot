use ocelot_ast::function_index::FunctionIndex;
use ocelot_base::shared_string::SharedString;
use std::collections::HashMap;

/// File-local semantic state used while resolving one module.
#[derive(Debug, Clone, Default)]
pub struct ModuleEnvironment {
    imported_function_symbols: HashMap<SharedString, FunctionIndex>,
}

impl ModuleEnvironment {
    /// Creates an empty module environment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves one imported function binding by file-local name.
    pub fn resolve_imported_function(&self, name: &str) -> Option<FunctionIndex> {
        self.imported_function_symbols.get(name).copied()
    }

    /// Registers one imported function binding for this module.
    pub fn add_imported_function(
        &mut self,
        local_name: impl Into<SharedString>,
        function_index: FunctionIndex,
    ) {
        self.imported_function_symbols
            .insert(local_name.into(), function_index);
    }
}
