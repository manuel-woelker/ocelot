use crate::item_kind::ItemKind;
use crate::trivia::Trivia;
use ocelot_base::span::Span;

/// Top-level source file item with an explicit source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub kind: ItemKind,
    pub trivia: Trivia,
    pub span: Span,
}

impl Item {
    /// Creates an item from its kind and source span.
    pub fn new(kind: ItemKind, span: Span) -> Self {
        Self::with_trivia(kind, Trivia::default(), span)
    }

    /// Creates an item from its kind, trivia, and source span.
    pub fn with_trivia(kind: ItemKind, trivia: Trivia, span: Span) -> Self {
        Self { kind, trivia, span }
    }
}
