use ocelot_ast::effect_index::EffectIndex;
use ocelot_ast::function_index::FunctionIndex;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::type_index::TypeIndex;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::function_kind::FunctionKind;
use crate::native_function::NativeFunction;

/// Definition record for one function entry in the symbol table.
#[derive(Debug, Clone)]
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
        module_name: impl Into<SharedString>,
        qualified_name: impl Into<SharedString>,
        argument_types: Vec<TypeIndex>,
        native_function: Box<dyn NativeFunction>,
        can_effects: BTreeSet<EffectIndex>,
        cannot_effects: BTreeSet<EffectIndex>,
    ) -> Self {
        Self {
            name: qualified_name.into(),
            module_name: module_name.into(),
            argument_types,
            can_effects: can_effects.clone(),
            cannot_effects,
            direct_effects: can_effects.clone(),
            direct_effect_sources: BTreeMap::new(),
            inferred_effects: can_effects,
            called_functions: BTreeMap::new(),
            can_clause_span: None,
            cannot_clause_span: None,
            kind: FunctionKind::NativeFunction { native_function },
        }
    }

    /// Creates a new user-defined function definition.
    pub fn user_defined(
        module_name: impl Into<SharedString>,
        qualified_name: impl Into<SharedString>,
        function: FunctionItem,
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
