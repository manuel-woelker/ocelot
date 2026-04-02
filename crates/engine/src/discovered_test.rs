use ocelot_base::shared_string::SharedString;
use ocelot_base::span::Span;

/// Metadata describing one discovered language-level test item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredTest {
    pub name: SharedString,
    pub span: Span,
}

impl DiscoveredTest {
    /// Creates a discovered test result.
    pub fn new(name: impl Into<SharedString>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}
