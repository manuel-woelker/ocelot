use crate::identifier::Identifier;
use ocelot_base::span::Span;

/// Top-level nominal effect declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectItem {
    pub identifier: Identifier,
    pub span: Span,
}

impl EffectItem {
    /// Creates one effect declaration item.
    pub fn new(identifier: Identifier, span: Span) -> Self {
        Self { identifier, span }
    }
}
