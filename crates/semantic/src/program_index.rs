use ocelot_ast::effect::Effect;
use ocelot_ast::effect_index::EffectIndex;
use ocelot_ast::function_index::FunctionIndex;
use ocelot_ast::ty::Ty;
use ocelot_ast::type_index::TypeIndex;
use ocelot_ast::type_kind::TypeKind;
use ocelot_base::result::{OcelotResult, OptionExt};
use ocelot_base::shared_string::SharedString;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::function_definition::FunctionDefinition;
use crate::function_kind::FunctionKind;
use crate::program_environment::ProgramEnvironment;

/// Immutable semantic index built from the declaration phase.
#[derive(Debug, Clone)]
pub struct ProgramIndex {
    pub functions: Vec<Option<FunctionDefinition>>,
    pub function_symbols: HashMap<SharedString, FunctionIndex>,
    pub module_symbols: HashSet<SharedString>,
    pub effects: Vec<Effect>,
    pub effect_symbols: HashMap<SharedString, EffectIndex>,
    pub types: Vec<Ty>,
    pub type_symbols: HashMap<SharedString, TypeIndex>,
}

impl Default for ProgramIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgramIndex {
    /// Creates a declaration index seeded with primitive types.
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

    /// Builds one immutable program index from the current program environment.
    pub fn from_environment(environment: &ProgramEnvironment) -> Self {
        Self {
            functions: environment.functions.clone(),
            function_symbols: environment.function_symbols.clone(),
            module_symbols: environment.module_symbols.clone(),
            effects: environment.effects.clone(),
            effect_symbols: environment.effect_symbols.clone(),
            types: environment.types.clone(),
            type_symbols: environment.type_symbols.clone(),
        }
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

    /// Resolves one function name to its table handle.
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
}
