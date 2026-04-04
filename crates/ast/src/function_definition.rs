use crate::effect_index::EffectIndex;
use crate::function_index::FunctionIndex;
use crate::function_kind::FunctionKind;
use crate::type_index::TypeIndex;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// Definition record for one function entry in the program environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    pub name: SharedString,
    pub module_name: SharedString,
    pub argument_types: Vec<TypeIndex>,
    pub can_effects: BTreeSet<EffectIndex>,
    pub cannot_effects: BTreeSet<EffectIndex>,
    pub direct_effects: BTreeSet<EffectIndex>,
    pub direct_effect_sources: BTreeMap<EffectIndex, Span>,
    pub inferred_effects: BTreeSet<EffectIndex>,
    pub called_functions: BTreeMap<FunctionIndex, Span>,
    pub can_clause_span: Option<Span>,
    pub cannot_clause_span: Option<Span>,
    pub kind: FunctionKind,
}

impl FunctionDefinition {
    /// Creates a new native function definition.
    pub fn native(
        name: impl Into<SharedString>,
        argument_types: Vec<TypeIndex>,
        native_function: crate::native_function::NativeFunction,
        effects: BTreeSet<EffectIndex>,
    ) -> Self {
        Self {
            name: name.into(),
            module_name: SharedString::empty(),
            argument_types,
            can_effects: BTreeSet::new(),
            cannot_effects: BTreeSet::new(),
            direct_effects: effects.clone(),
            direct_effect_sources: BTreeMap::new(),
            inferred_effects: effects,
            called_functions: BTreeMap::new(),
            can_clause_span: None,
            cannot_clause_span: None,
            kind: FunctionKind::Native { native_function },
        }
    }

    /// Creates a new user-defined function definition.
    pub fn user_defined(
        module_name: impl Into<SharedString>,
        qualified_name: impl Into<SharedString>,
        function: crate::function_item::FunctionItem,
        argument_types: Vec<TypeIndex>,
        can_effects: BTreeSet<EffectIndex>,
        cannot_effects: BTreeSet<EffectIndex>,
        source_file: SourceFile,
    ) -> Self {
        Self {
            name: qualified_name.into(),
            module_name: module_name.into(),
            argument_types,
            direct_effects: can_effects.clone(),
            direct_effect_sources: BTreeMap::new(),
            inferred_effects: can_effects.clone(),
            called_functions: BTreeMap::new(),
            can_clause_span: function
                .can_clause
                .as_ref()
                .map(|clause| clause.span.clone()),
            cannot_clause_span: function
                .cannot_clause
                .as_ref()
                .map(|clause| clause.span.clone()),
            can_effects,
            cannot_effects,
            kind: FunctionKind::UserDefined {
                function: Box::new(function),
                source_file: Box::new(source_file),
            },
        }
    }
}
