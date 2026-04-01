use crate::statement::Statement;
use ocelot_base::span::Span;

/// Root syntax node for a script-like `ocelot` source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub statements: Vec<Statement>,
    pub span: Span,
}

impl Script {
    /// Creates a script from its statements and source span.
    pub fn new(statements: Vec<Statement>, span: Span) -> Self {
        Self { statements, span }
    }
}

impl Default for Script {
    fn default() -> Self {
        Self::new(Vec::new(), Span::default())
    }
}
