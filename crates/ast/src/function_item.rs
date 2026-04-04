use crate::identifier::Identifier;
use crate::statement::Statement;
use ocelot_base::span::Span;

/// Top-level user-defined function declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionItem {
    pub identifier: Identifier,
    pub body: Vec<Statement>,
    pub span: Span,
}

impl FunctionItem {
    /// Creates a function item from its identifier, body, and source span.
    pub fn new(identifier: Identifier, body: Vec<Statement>, span: Span) -> Self {
        Self {
            identifier,
            body,
            span,
        }
    }
}
