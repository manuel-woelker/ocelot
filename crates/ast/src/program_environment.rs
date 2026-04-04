use crate::function_definition::FunctionDefinition;
use crate::function_index::FunctionIndex;
use crate::function_item::FunctionItem;
use crate::function_kind::FunctionKind;
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

    /// Returns the definition for one previously resolved function index mutably.
    pub fn function_definition_mut(
        &mut self,
        function_index: FunctionIndex,
    ) -> OcelotResult<&mut FunctionDefinition> {
        self.functions
            .get_mut(function_index.as_usize())
            .and_then(Option::as_mut)
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

    /// Returns the indices of all user-defined functions in insertion order.
    pub fn user_defined_function_indices(&self) -> Vec<FunctionIndex> {
        self.functions
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(index, function)| {
                let function = function.as_ref()?;
                matches!(function.kind, FunctionKind::UserDefined { .. })
                    .then(|| FunctionIndex::new(index as u32))
            })
            .collect()
    }

    /// Removes one user-defined function body from the table temporarily.
    pub fn take_user_defined_function(
        &mut self,
        function_index: FunctionIndex,
    ) -> OcelotResult<Box<FunctionItem>> {
        let function_definition = self.function_definition_mut(function_index)?;
        let FunctionKind::UserDefined { function } = &mut function_definition.kind else {
            ocelot_base::bail!(
                "internal error: function index did not reference a user-defined function"
            );
        };

        Ok(std::mem::replace(
            function,
            Box::new(FunctionItem::new(
                "",
                Vec::new(),
                ocelot_base::span::Span::default(),
            )),
        ))
    }

    /// Writes one user-defined function body back into the table.
    pub fn put_user_defined_function(
        &mut self,
        function_index: FunctionIndex,
        function: Box<FunctionItem>,
    ) -> OcelotResult<()> {
        let function_definition = self.function_definition_mut(function_index)?;
        let FunctionKind::UserDefined {
            function: stored_function,
        } = &mut function_definition.kind
        else {
            ocelot_base::bail!(
                "internal error: function index did not reference a user-defined function"
            );
        };

        *stored_function = function;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ProgramEnvironment;
    use crate::function_definition::FunctionDefinition;
    use crate::function_index::FunctionIndex;
    use crate::function_item::FunctionItem;
    use crate::function_kind::FunctionKind;
    use crate::native_function::NativeFunction;
    use ocelot_base::span::Span;
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

        let function_index = environment.add_function(FunctionDefinition::user_defined(
            FunctionItem::new("greet", Vec::new(), Span::new(0, 13)),
        ));

        assert_eq!(environment.resolve_function("greet"), Some(function_index));
        assert!(matches!(
            environment
                .function_definition(function_index)
                .unwrap()
                .kind,
            FunctionKind::UserDefined { .. }
        ));
    }

    #[test]
    fn user_defined_function_indices_returns_only_user_defined_entries() {
        let environment = ProgramEnvironment::new(vec![
            FunctionDefinition::native("println", NativeFunction::Println),
            FunctionDefinition::user_defined(FunctionItem::new(
                "greet",
                Vec::new(),
                Span::new(0, 13),
            )),
            FunctionDefinition::native("assert", NativeFunction::Assert),
        ]);

        assert_eq!(
            environment.user_defined_function_indices(),
            vec![FunctionIndex::new(2)]
        );
    }

    #[test]
    fn take_and_put_user_defined_function_round_trips() {
        let mut environment = ProgramEnvironment::new(vec![FunctionDefinition::user_defined(
            FunctionItem::new("greet", Vec::new(), Span::new(0, 13)),
        )]);
        let function_index = FunctionIndex::new(1);

        let function = environment
            .take_user_defined_function(function_index)
            .unwrap();
        assert_eq!(function.name, "greet");

        environment
            .put_user_defined_function(function_index, function)
            .unwrap();
        assert!(matches!(
            environment
                .function_definition(function_index)
                .unwrap()
                .kind,
            FunctionKind::UserDefined { .. }
        ));
    }
}
