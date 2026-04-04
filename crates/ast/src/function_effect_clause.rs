use crate::identifier::Identifier;
use ocelot_base::span::Span;

/// One parsed function effect clause such as `can exec` or `cannot panic`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionEffectClause {
    pub effect: Identifier,
    pub span: Span,
}

impl FunctionEffectClause {
    /// Creates one function effect clause from its effect identifier and source span.
    pub fn new(effect: Identifier, span: Span) -> Self {
        Self { effect, span }
    }
}
