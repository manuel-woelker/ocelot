use crate::statement::Statement;
use ocelot_base::shared_string::SharedString;
use ocelot_base::span::Span;

/// Top-level user-defined function declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionItem {
    pub name: SharedString,
    pub name_span: Span,
    pub body: Vec<Statement>,
    pub span: Span,
}

impl FunctionItem {
    /// Creates a function item from its name, body, and source span.
    pub fn new(
        name: impl Into<SharedString>,
        name_span: Span,
        body: Vec<Statement>,
        span: Span,
    ) -> Self {
        Self {
            name: name.into(),
            name_span,
            body,
            span,
        }
    }
}
