use ocelot_base::shared_string::SharedString;
use ocelot_base::span::Span;

/// Identifier with source span information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    pub name: SharedString,
    pub span: Span,
}

impl Identifier {
    /// Creates an identifier from its name and source span.
    pub fn new(name: impl Into<SharedString>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}
