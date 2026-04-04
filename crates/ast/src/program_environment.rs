use crate::function_definition::FunctionDefinition;
use crate::function_index::FunctionIndex;
use crate::native_function::NativeFunction;
use ocelot_base::result::{OcelotResult, OptionExt};
use ocelot_base::shared_string::SharedString;
use std::collections::HashMap;

/// Shared program-level data needed by resolution and interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramEnvironment {
    pub functions: Vec<FunctionDefinition>,
    pub function_symbols: HashMap<SharedString, FunctionIndex>,
}

impl ProgramEnvironment {
    /// Creates a program environment from the provided function definitions.
    pub fn new(functions: Vec<FunctionDefinition>) -> Self {
        let function_symbols = functions
            .iter()
            .enumerate()
            .map(|(index, function)| (function.name.clone(), FunctionIndex::new(index as u32)))
            .collect();

        Self {
            functions,
            function_symbols,
        }
    }

    /// Creates the default environment for the current language slice.
    pub fn native() -> Self {
        Self::new(vec![
            FunctionDefinition::new("println", NativeFunction::Println),
            FunctionDefinition::new("assert", NativeFunction::Assert),
            FunctionDefinition::new("assert_eq", NativeFunction::AssertEq),
        ])
    }

    /// Resolves one function name to its table handle.
    pub fn resolve_function(&self, name: &str) -> Option<FunctionIndex> {
        self.function_symbols.get(name).copied()
    }

    /// Returns the definition for one previously resolved function index.
    pub fn function_definition(
        &self,
        function_index: FunctionIndex,
    ) -> OcelotResult<&FunctionDefinition> {
        self.functions
            .get(function_index.as_usize())
            .context("internal error: function index points outside the function table")
    }
}

impl Default for ProgramEnvironment {
    fn default() -> Self {
        Self::native()
    }
}

#[cfg(test)]
mod tests {
    use super::ProgramEnvironment;
    use crate::native_function::NativeFunction;

    #[test]
    fn native_environment_contains_the_current_native_functions() {
        let environment = ProgramEnvironment::native();

        assert!(environment.resolve_function("println").is_some());
        assert!(environment.resolve_function("assert").is_some());
        assert!(environment.resolve_function("assert_eq").is_some());
    }

    #[test]
    fn function_definition_looks_up_native_function_metadata() {
        let environment = ProgramEnvironment::native();
        let function_index = environment.resolve_function("println").unwrap();
        let function = environment.function_definition(function_index).unwrap();

        assert_eq!(function.name, "println");
        assert_eq!(function.native_function, NativeFunction::Println);
    }
}
