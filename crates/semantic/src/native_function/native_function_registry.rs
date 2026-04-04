use std::collections::HashMap;

use ocelot_base::shared_string::SharedString;

use super::native_function_trait::NativeFunction;

/// Compiler-provided registry of native implementations keyed by fully qualified name.
#[derive(Debug, Clone, Default)]
pub struct NativeFunctionRegistry {
    functions: HashMap<SharedString, Box<dyn NativeFunction>>,
}

impl NativeFunctionRegistry {
    /// Creates an empty native function registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one native implementation under its fully qualified name.
    pub fn register(
        &mut self,
        qualified_name: impl Into<SharedString>,
        native_function: Box<dyn NativeFunction>,
    ) {
        self.functions
            .insert(qualified_name.into(), native_function);
    }

    /// Resolves one native implementation by fully qualified name.
    pub fn resolve(&self, qualified_name: &str) -> Option<Box<dyn NativeFunction>> {
        self.functions.get(qualified_name).cloned()
    }
}
