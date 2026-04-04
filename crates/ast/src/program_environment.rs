use crate::function_definition::FunctionDefinition;
use crate::function_index::FunctionIndex;
use ocelot_base::result::{OcelotResult, OptionExt};
use ocelot_base::shared_string::SharedString;
use std::collections::HashMap;

/// Shared program-level data needed by resolution and interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramEnvironment {
    pub functions: Vec<Option<FunctionDefinition>>,
    pub function_symbols: HashMap<SharedString, FunctionIndex>,
}

impl ProgramEnvironment {
    /// Creates a program environment from the provided function definitions.
    pub fn new(functions: Vec<FunctionDefinition>) -> Self {
        let function_symbols = functions
            .iter()
            .enumerate()
            .map(|(index, function)| {
                (
                    function.name.clone(),
                    FunctionIndex::new((index + 1) as u32),
                )
            })
            .collect();

        let mut table = vec![None];
        table.extend(functions.into_iter().map(Some));

        Self {
            functions: table,
            function_symbols,
        }
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
            .and_then(Option::as_ref)
            .context("internal error: function index points outside the function table")
    }

    /// Appends one new function definition and returns its table handle.
    pub fn add_function(&mut self, function: FunctionDefinition) -> FunctionIndex {
        let function_index = FunctionIndex::new(self.functions.len() as u32);
        self.function_symbols
            .insert(function.name.clone(), function_index);
        self.functions.push(Some(function));
        function_index
    }
}

#[cfg(test)]
mod tests {
    use super::ProgramEnvironment;
    use crate::function_definition::FunctionDefinition;
    use crate::function_kind::FunctionKind;
    use crate::native_function::NativeFunction;
    #[test]
    fn program_environment_indexes_functions_by_name() {
        let environment = ProgramEnvironment::new(vec![
            FunctionDefinition::native("println", NativeFunction::Println),
            FunctionDefinition::native("assert", NativeFunction::Assert),
            FunctionDefinition::native("assert_eq", NativeFunction::AssertEq),
        ]);

        assert!(environment.resolve_function("println").is_some());
        assert!(environment.resolve_function("assert").is_some());
        assert!(environment.resolve_function("assert_eq").is_some());
    }

    #[test]
    fn function_definition_looks_up_native_function_metadata() {
        let environment = ProgramEnvironment::new(vec![FunctionDefinition::native(
            "println",
            NativeFunction::Println,
        )]);
        let function_index = environment.resolve_function("println").unwrap();
        let function = environment.function_definition(function_index).unwrap();

        assert_eq!(function.name, "println");
        assert_eq!(
            function.kind,
            FunctionKind::Native {
                native_function: NativeFunction::Println,
            }
        );
    }

    #[test]
    fn program_environment_reserves_table_entry_zero() {
        let environment = ProgramEnvironment::new(vec![FunctionDefinition::native(
            "println",
            NativeFunction::Println,
        )]);

        assert!(environment.functions[0].is_none());
    }

    #[test]
    fn add_function_appends_user_defined_entries() {
        let mut environment = ProgramEnvironment::new(vec![FunctionDefinition::native(
            "println",
            NativeFunction::Println,
        )]);

        let function_index = environment.add_function(FunctionDefinition::user_defined("greet", 3));

        assert_eq!(environment.resolve_function("greet"), Some(function_index));
        assert_eq!(
            environment
                .function_definition(function_index)
                .unwrap()
                .kind,
            FunctionKind::UserDefined { item_index: 3 }
        );
    }
}
