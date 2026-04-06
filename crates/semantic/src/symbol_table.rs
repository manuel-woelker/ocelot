use ocelot_ast::effect::Effect;
use ocelot_ast::effect_index::EffectIndex;
use ocelot_ast::function_index::FunctionIndex;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::identifier::Identifier;
use ocelot_ast::ty::Ty;
use ocelot_ast::type_index::TypeIndex;
use ocelot_ast::type_kind::TypeKind;
use ocelot_base::result::{OcelotResult, OptionExt};
use ocelot_base::shared_string::SharedString;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::function_definition::FunctionDefinition;
use crate::function_kind::FunctionKind;
use crate::resolved_function::ResolvedFunction;

/// Canonical program-wide semantic table used by resolution and execution.
#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub functions: Vec<Option<FunctionDefinition>>,
    pub function_symbols: HashMap<SharedString, FunctionIndex>,
    pub module_symbols: HashSet<SharedString>,
    pub effects: Vec<Effect>,
    pub effect_symbols: HashMap<SharedString, EffectIndex>,
    pub types: Vec<Ty>,
    pub type_symbols: HashMap<SharedString, TypeIndex>,
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    /// Creates a symbol table seeded with primitive types.
    pub fn new() -> Self {
        let mut index = Self {
            functions: vec![None],
            function_symbols: HashMap::new(),
            module_symbols: HashSet::new(),
            effects: vec![Effect::builtin("__reserved_effect_slot__")],
            effect_symbols: HashMap::new(),
            types: vec![Ty::new("unresolved", TypeKind::Unresolved)],
            type_symbols: HashMap::new(),
        };
        index.seed_builtin_types();
        index
    }

    fn seed_builtin_types(&mut self) {
        self.add_type(Ty::new("any", TypeKind::Any));
        self.add_type(Ty::new("string", TypeKind::String));
        self.add_type(Ty::new("bool", TypeKind::Boolean));
    }

    /// Resolves one effect name to its table handle.
    pub fn resolve_effect(&self, name: &str) -> Option<EffectIndex> {
        self.effect_symbols.get(name).copied()
    }

    /// Returns the definition for one effect index.
    pub fn effect_definition(&self, effect_index: EffectIndex) -> OcelotResult<&Effect> {
        self.effects
            .get(effect_index.as_usize())
            .context("internal error: effect index points outside the effect table")
    }

    /// Appends one new effect definition and returns its table handle.
    pub fn add_effect(&mut self, effect: Effect) -> EffectIndex {
        let effect_index = EffectIndex::new(self.effects.len() as u32);
        self.effect_symbols
            .insert(effect.name.clone(), effect_index);
        self.effects.push(effect);
        effect_index
    }

    /// Returns the canonical write-stdout effect handle.
    pub fn write_stdout_effect_index(&self) -> EffectIndex {
        self.resolve_effect("write_stdout")
            .expect("write_stdout effect should already be declared")
    }

    /// Returns the canonical panic effect handle.
    pub fn panic_effect_index(&self) -> EffectIndex {
        self.resolve_effect("panic")
            .expect("panic effect should already be declared")
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

    /// Returns the canonical any type handle.
    pub fn any_type_index(&self) -> TypeIndex {
        self.resolve_type("any")
            .expect("any type should always be seeded")
    }

    /// Returns the canonical string type handle.
    pub fn string_type_index(&self) -> TypeIndex {
        self.resolve_type("string")
            .expect("string type should always be seeded")
    }

    /// Returns the canonical boolean type handle.
    pub fn boolean_type_index(&self) -> TypeIndex {
        self.resolve_type("bool")
            .expect("bool type should always be seeded")
    }

    /// Returns the canonical unresolved type handle.
    pub fn unresolved_type_index(&self) -> TypeIndex {
        TypeIndex::unresolved()
    }

    /// Resolves one function name to its table handle.
    pub fn resolve_function(&self, name: &str) -> Option<FunctionIndex> {
        self.function_symbols.get(name).copied()
    }

    /// Resolves one function name without module fallback.
    pub fn resolve_function_exact(&self, name: &str) -> Option<FunctionIndex> {
        self.function_symbols.get(name).copied()
    }

    /// Registers one module name.
    pub fn add_module(&mut self, module_name: impl Into<SharedString>) {
        self.module_symbols.insert(module_name.into());
    }

    /// Returns whether one module name is known.
    pub fn has_module(&self, module_name: &str) -> bool {
        self.module_symbols.contains(module_name)
    }

    /// Creates a qualified function name from a module and local name.
    pub fn qualify_function_name(&self, module_name: &str, function_name: &str) -> SharedString {
        if module_name.is_empty() {
            return function_name.into();
        }

        format!("{module_name}::{function_name}").into()
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

    /// Applies resolved bodies and effect metadata for user-defined functions.
    pub fn apply_resolved_functions(
        &mut self,
        resolved_functions: Vec<ResolvedFunction>,
    ) -> OcelotResult<()> {
        for resolved_function in resolved_functions {
            let function_definition =
                self.function_definition_mut(resolved_function.function_index)?;
            let FunctionKind::UserDefined {
                function: stored_function,
                ..
            } = &mut function_definition.kind
            else {
                ocelot_base::bail!(
                    "internal error: function index did not reference a user-defined function"
                );
            };

            *stored_function = resolved_function.function;
            function_definition.direct_effects = resolved_function.direct_effects;
            function_definition.direct_effect_sources = resolved_function.direct_effect_sources;
            function_definition.inferred_effects = resolved_function.inferred_effects;
            function_definition.called_functions = resolved_function.called_functions;
        }

        Ok(())
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
        let FunctionKind::UserDefined { function, .. } = &mut function_definition.kind else {
            ocelot_base::bail!(
                "internal error: function index did not reference a user-defined function"
            );
        };

        Ok(std::mem::replace(
            function,
            Box::new(FunctionItem::new(
                Identifier::new("", ocelot_base::span::Span::default()),
                Vec::new(),
                None,
                None,
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
            ..
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
    use super::SymbolTable;
    use ocelot_ast::effect::Effect;
    use ocelot_ast::function_index::FunctionIndex;
    use ocelot_ast::function_item::FunctionItem;
    use ocelot_ast::identifier::Identifier;
    use ocelot_ast::type_index::TypeIndex;
    use ocelot_ast::type_kind::TypeKind;
    use ocelot_base::source_file::SourceFile;
    use ocelot_base::span::Span;
    use std::collections::BTreeSet;

    use crate::function_definition::FunctionDefinition;
    use crate::function_kind::FunctionKind;

    #[test]
    fn add_function_can_store_native_function_metadata() {
        let mut symbol_table = SymbolTable::new();
        let function_index = symbol_table.add_function(FunctionDefinition::native(
            "core",
            "core::println",
            vec![symbol_table.any_type_index()],
            crate::native_function::default_native_function_registry()
                .resolve("core::println")
                .unwrap(),
            BTreeSet::new(),
            BTreeSet::new(),
        ));
        let function = symbol_table.function_definition(function_index).unwrap();

        assert_eq!(function.name, "core::println");
        assert_eq!(function.module_name, "core");
        assert_eq!(function.argument_types, vec![symbol_table.any_type_index()]);
        assert!(matches!(
            function.kind,
            crate::function_kind::FunctionKind::NativeFunction { .. }
        ));
    }

    #[test]
    fn symbol_table_reserves_table_entry_zero() {
        let symbol_table = SymbolTable::new();

        assert!(symbol_table.functions[0].is_none());
    }

    #[test]
    fn symbol_table_seeds_primitive_types() {
        let symbol_table = SymbolTable::new();

        assert_eq!(
            symbol_table
                .type_definition(TypeIndex::unresolved())
                .unwrap()
                .kind,
            TypeKind::Unresolved
        );
        assert_eq!(
            symbol_table
                .type_definition(symbol_table.resolve_type("any").unwrap())
                .unwrap()
                .kind,
            TypeKind::Any
        );
        assert_eq!(
            symbol_table
                .type_definition(symbol_table.resolve_type("string").unwrap())
                .unwrap()
                .kind,
            TypeKind::String
        );
        assert_eq!(
            symbol_table
                .type_definition(symbol_table.resolve_type("bool").unwrap())
                .unwrap()
                .kind,
            TypeKind::Boolean
        );
    }

    #[test]
    fn symbol_table_indexes_types_by_name() {
        let symbol_table = SymbolTable::new();

        assert_eq!(symbol_table.resolve_type("any"), Some(TypeIndex::new(1)));
        assert_eq!(symbol_table.resolve_type("string"), Some(TypeIndex::new(2)));
        assert_eq!(symbol_table.resolve_type("bool"), Some(TypeIndex::new(3)));
    }

    #[test]
    fn add_effect_indexes_effects_by_name() {
        let mut symbol_table = SymbolTable::new();

        let write_stdout = symbol_table.add_effect(Effect::builtin("write_stdout"));
        let panic = symbol_table.add_effect(Effect::builtin("panic"));

        assert_eq!(
            symbol_table.effect_definition(write_stdout).unwrap().name,
            "write_stdout"
        );
        assert_eq!(symbol_table.effect_definition(panic).unwrap().name, "panic");
    }

    #[test]
    fn write_stdout_and_panic_effect_accessors_use_declared_effects() {
        let mut symbol_table = SymbolTable::new();
        let write_stdout = symbol_table.add_effect(Effect::builtin("write_stdout"));
        let panic = symbol_table.add_effect(Effect::builtin("panic"));

        assert_eq!(
            symbol_table.resolve_effect("write_stdout"),
            Some(write_stdout)
        );
        assert_eq!(symbol_table.resolve_effect("panic"), Some(panic));
        assert_eq!(symbol_table.write_stdout_effect_index(), write_stdout);
        assert_eq!(symbol_table.panic_effect_index(), panic);
    }

    #[test]
    fn add_function_appends_user_defined_entries() {
        let mut symbol_table = SymbolTable::new();

        let function_index = symbol_table.add_function(FunctionDefinition::user_defined(
            "greetings",
            "greetings::greet",
            FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                Vec::new(),
                None,
                None,
                Vec::new(),
                Span::new(0, 13),
            ),
            Vec::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            SourceFile::new("greetings.ocelot", "fun greet() {}"),
        ));

        assert_eq!(
            symbol_table.resolve_function("greetings::greet"),
            Some(function_index)
        );
        assert!(matches!(
            symbol_table
                .function_definition(function_index)
                .unwrap()
                .kind,
            FunctionKind::UserDefined { .. }
        ));
    }

    #[test]
    fn user_defined_function_indices_returns_only_user_defined_entries() {
        let mut symbol_table = SymbolTable::new();
        let _ = symbol_table.add_function(FunctionDefinition::user_defined(
            "greetings",
            "greetings::greet",
            FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                Vec::new(),
                None,
                None,
                Vec::new(),
                Span::new(0, 13),
            ),
            Vec::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            SourceFile::new("greetings.ocelot", "fun greet() {}"),
        ));

        assert_eq!(
            symbol_table.user_defined_function_indices(),
            vec![FunctionIndex::new(1)]
        );
    }

    #[test]
    fn take_and_put_user_defined_function_round_trips() {
        let mut symbol_table = SymbolTable::new();
        let function_index = symbol_table.add_function(FunctionDefinition::user_defined(
            "greetings",
            "greetings::greet",
            FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                Vec::new(),
                None,
                None,
                Vec::new(),
                Span::new(0, 13),
            ),
            Vec::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            SourceFile::new("greetings.ocelot", "fun greet() {}"),
        ));

        let function = symbol_table
            .take_user_defined_function(function_index)
            .unwrap();
        assert_eq!(function.identifier.name, "greet");

        symbol_table
            .put_user_defined_function(function_index, function)
            .unwrap();
        assert!(matches!(
            symbol_table
                .function_definition(function_index)
                .unwrap()
                .kind,
            FunctionKind::UserDefined { .. }
        ));
    }

    #[test]
    fn resolve_function_exact_looks_up_qualified_names() {
        let mut symbol_table = SymbolTable::new();
        let function_index = symbol_table.add_function(FunctionDefinition::user_defined(
            "math",
            "math::greet",
            FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                Vec::new(),
                None,
                None,
                Vec::new(),
                Span::new(0, 13),
            ),
            Vec::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            SourceFile::new("math.ocelot", "fun greet() {}"),
        ));

        assert_eq!(
            symbol_table.resolve_function_exact("math::greet"),
            Some(function_index)
        );
        assert!(
            symbol_table
                .resolve_function_exact("other::greet")
                .is_none()
        );
        assert_eq!(
            symbol_table.resolve_function("math::greet"),
            Some(function_index)
        );
    }
}
