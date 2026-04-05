use crate::statement_kind::StatementKind;
use ocelot_base::span::Span;

/// Source-file statement with an explicit source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
}

impl Statement {
    /// Creates a statement from its kind and source span.
    pub fn new(kind: StatementKind, span: Span) -> Self {
        Self { kind, span }
    }
}
