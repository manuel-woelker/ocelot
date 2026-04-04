use crate::function_definition::FunctionDefinition;
use crate::function_index::FunctionIndex;
use crate::function_item::FunctionItem;
use crate::function_kind::FunctionKind;
use crate::identifier::Identifier;
use crate::native_function::NativeFunction;
use crate::ty::Ty;
use crate::type_index::TypeIndex;
use crate::type_kind::TypeKind;
use ocelot_base::result::{OcelotResult, OptionExt};
use ocelot_base::shared_string::SharedString;
use std::collections::HashMap;

/// Shared program-level data needed by resolution and interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramEnvironment {
    pub functions: Vec<Option<FunctionDefinition>>,
    pub function_symbols: HashMap<SharedString, FunctionIndex>,
    pub types: Vec<Ty>,
    pub type_symbols: HashMap<SharedString, TypeIndex>,
}

impl Default for ProgramEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgramEnvironment {
    /// Creates a program environment seeded with builtin types and native functions.
    pub fn new() -> Self {
        let mut environment = Self {
            functions: vec![None],
            function_symbols: HashMap::new(),
            types: vec![Ty::new("unresolved", TypeKind::Unresolved)],
            type_symbols: HashMap::new(),
        };

        environment.seed_builtin_types();
        environment.seed_native_functions();
        environment
    }

    fn seed_builtin_types(&mut self) {
        self.add_type(Ty::new("any", TypeKind::Any));
        self.add_type(Ty::new("string", TypeKind::String));
        self.add_type(Ty::new("boolean", TypeKind::Boolean));
    }

    fn seed_native_functions(&mut self) {
        self.add_function(FunctionDefinition::native(
            "println",
            vec![self.any_type_index()],
            NativeFunction::Println,
        ));
        self.add_function(FunctionDefinition::native(
            "assert",
            vec![self.boolean_type_index()],
            NativeFunction::Assert,
        ));
        self.add_function(FunctionDefinition::native(
            "assert_eq",
            vec![self.any_type_index(), self.any_type_index()],
            NativeFunction::AssertEq,
        ));
    }

    /// Resolves one type name to its table handle.
    pub fn resolve_type(&self, name: &str) -> Option<TypeIndex> {
        self.type_symbols.get(name).copied()
    }

    /// Returns the definition for one type index.
    pub fn type_definition(&self, type_index: TypeIndex) -> OcelotResult<&Ty> {
        self.types
            .get(type_index.as_usize())
            .context("internal error: type index points outside the type table")
    }

    /// Appends one new type definition and returns its table handle.
    pub fn add_type(&mut self, ty: Ty) -> TypeIndex {
        let type_index = TypeIndex::new(self.types.len() as u32);
        self.type_symbols.insert(ty.name.clone(), type_index);
        self.types.push(ty);
        type_index
    }

    /// Returns the canonical string type handle.
    pub fn string_type_index(&self) -> TypeIndex {
        self.resolve_type("string")
            .expect("string type should always be seeded")
    }

    /// Returns the canonical any type handle.
    pub fn any_type_index(&self) -> TypeIndex {
        self.resolve_type("any")
            .expect("any type should always be seeded")
    }

    /// Returns the canonical boolean type handle.
    pub fn boolean_type_index(&self) -> TypeIndex {
        self.resolve_type("boolean")
            .expect("boolean type should always be seeded")
    }

    /// Returns the canonical unresolved type handle.
    pub fn unresolved_type_index(&self) -> TypeIndex {
        TypeIndex::unresolved()
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
                Identifier::new("", ocelot_base::span::Span::default()),
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
    use crate::identifier::Identifier;
    use crate::native_function::NativeFunction;
    use crate::type_index::TypeIndex;
    use crate::type_kind::TypeKind;
    use ocelot_base::span::Span;
    #[test]
    fn program_environment_indexes_functions_by_name() {
        let environment = ProgramEnvironment::new();

        assert!(environment.resolve_function("println").is_some());
        assert!(environment.resolve_function("assert").is_some());
        assert!(environment.resolve_function("assert_eq").is_some());
    }

    #[test]
    fn function_definition_looks_up_native_function_metadata() {
        let environment = ProgramEnvironment::new();
        let function_index = environment.resolve_function("println").unwrap();
        let function = environment.function_definition(function_index).unwrap();

        assert_eq!(function.name, "println");
        assert_eq!(function.argument_types, vec![environment.any_type_index()]);
        assert_eq!(
            function.kind,
            FunctionKind::Native {
                native_function: NativeFunction::Println,
            }
        );
    }

    #[test]
    fn program_environment_reserves_table_entry_zero() {
        let environment = ProgramEnvironment::new();

        assert!(environment.functions[0].is_none());
    }

    #[test]
    fn program_environment_seeds_primitive_types() {
        let environment = ProgramEnvironment::new();

        assert_eq!(
            environment
                .type_definition(TypeIndex::unresolved())
                .unwrap()
                .kind,
            TypeKind::Unresolved
        );
        assert_eq!(
            environment
                .type_definition(environment.resolve_type("any").unwrap())
                .unwrap()
                .kind,
            TypeKind::Any
        );
        assert_eq!(
            environment
                .type_definition(environment.resolve_type("string").unwrap())
                .unwrap()
                .kind,
            TypeKind::String
        );
        assert_eq!(
            environment
                .type_definition(environment.resolve_type("boolean").unwrap())
                .unwrap()
                .kind,
            TypeKind::Boolean
        );
    }

    #[test]
    fn program_environment_indexes_types_by_name() {
        let environment = ProgramEnvironment::new();

        assert_eq!(environment.resolve_type("any"), Some(TypeIndex::new(1)));
        assert_eq!(environment.resolve_type("string"), Some(TypeIndex::new(2)));
        assert_eq!(environment.resolve_type("boolean"), Some(TypeIndex::new(3)));
    }

    #[test]
    fn add_function_appends_user_defined_entries() {
        let mut environment = ProgramEnvironment::new();

        let function_index =
            environment.add_function(FunctionDefinition::user_defined(FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                Vec::new(),
                Span::new(0, 13),
            )));

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
        let mut environment = ProgramEnvironment::new();
        let _ = environment.add_function(FunctionDefinition::user_defined(FunctionItem::new(
            Identifier::new("greet", Span::new(4, 9)),
            Vec::new(),
            Span::new(0, 13),
        )));

        assert_eq!(
            environment.user_defined_function_indices(),
            vec![FunctionIndex::new(4)]
        );
    }

    #[test]
    fn take_and_put_user_defined_function_round_trips() {
        let mut environment = ProgramEnvironment::new();
        let function_index =
            environment.add_function(FunctionDefinition::user_defined(FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                Vec::new(),
                Span::new(0, 13),
            )));

        let function = environment
            .take_user_defined_function(function_index)
            .unwrap();
        assert_eq!(function.identifier.name, "greet");

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
