use crate::native_function::NativeFunctionRegistry;
use crate::native_function::default_native_function_registry;

/// External compiler inputs for one compilation run.
#[derive(Debug, Clone, Default)]
pub struct CompilationSession {
    native_function_registry: NativeFunctionRegistry,
}

impl CompilationSession {
    /// Creates one empty compilation session.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates one compilation session with the default native function registry.
    pub fn with_default_native_functions() -> Self {
        Self {
            native_function_registry: default_native_function_registry(),
        }
    }

    /// Returns the native function registry for this compilation.
    pub fn native_function_registry(&self) -> &NativeFunctionRegistry {
        &self.native_function_registry
    }
}
