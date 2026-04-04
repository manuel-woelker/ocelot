use ocelot_ast::effect_index::EffectIndex;
use ocelot_ast::function_index::FunctionIndex;
use ocelot_ast::function_item::FunctionItem;
use ocelot_base::span::Span;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// Resolved body and effect metadata for one user-defined function.
#[derive(Debug, Clone)]
pub struct ResolvedFunction {
    pub function_index: FunctionIndex,
    pub function: Box<FunctionItem>,
    pub direct_effects: BTreeSet<EffectIndex>,
    pub direct_effect_sources: BTreeMap<EffectIndex, Span>,
    pub inferred_effects: BTreeSet<EffectIndex>,
    pub called_functions: BTreeMap<FunctionIndex, Span>,
}

impl ResolvedFunction {
    /// Creates one resolved function payload with the given initial direct effects.
    pub fn new(
        function_index: FunctionIndex,
        function: FunctionItem,
        direct_effects: BTreeSet<EffectIndex>,
    ) -> Self {
        Self {
            function_index,
            function: Box::new(function),
            direct_effects: direct_effects.clone(),
            direct_effect_sources: BTreeMap::new(),
            inferred_effects: direct_effects,
            called_functions: BTreeMap::new(),
        }
    }
}
