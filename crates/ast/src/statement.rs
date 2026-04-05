use crate::statement_kind::StatementKind;
use crate::trivia::Trivia;
use ocelot_base::span::Span;

/// Source-file statement with an explicit source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub kind: StatementKind,
    pub trivia: Trivia,
    pub span: Span,
}

impl Statement {
    /// Creates a statement from its kind and source span.
    pub fn new(kind: StatementKind, span: Span) -> Self {
        Self::with_trivia(kind, Trivia::default(), span)
    }

    /// Creates a statement from its kind, trivia, and source span.
    pub fn with_trivia(kind: StatementKind, trivia: Trivia, span: Span) -> Self {
        Self { kind, trivia, span }
    }
}
