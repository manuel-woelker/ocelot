use crate::item_kind::ItemKind;
use ocelot_base::span::Span;

/// Top-level source file item with an explicit source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

impl Item {
    /// Creates an item from its kind and source span.
    pub fn new(kind: ItemKind, span: Span) -> Self {
        Self { kind, span }
    }
}
