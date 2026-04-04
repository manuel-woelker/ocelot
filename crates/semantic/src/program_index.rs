use ocelot_ast::effect::Effect;
use ocelot_ast::effect_index::EffectIndex;
use ocelot_ast::function_index::FunctionIndex;
use ocelot_ast::ty::Ty;
use ocelot_ast::type_index::TypeIndex;
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

impl ProgramIndex {
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
