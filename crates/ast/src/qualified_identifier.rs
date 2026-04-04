use crate::identifier::Identifier;
use ocelot_base::shared_string::SharedString;
use ocelot_base::span::Span;

/// Source-level `::`-qualified identifier path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedIdentifier {
    pub segments: Vec<Identifier>,
}

impl QualifiedIdentifier {
    /// Creates a qualified identifier from its path segments.
    pub fn new(segments: Vec<Identifier>) -> Self {
        Self { segments }
    }

    /// Returns the span covering the full qualified identifier.
    pub fn span(&self) -> Span {
        let start = self
            .segments
            .first()
            .map(|segment| segment.span.start())
            .unwrap_or_default();
        let end = self
            .segments
            .last()
            .map(|segment| segment.span.end())
            .unwrap_or_default();
        Span::new(start, end)
    }

    /// Renders the identifier path using `::` separators.
    pub fn render(&self) -> SharedString {
        self.segments
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join("::")
            .into()
    }

    /// Returns the final path segment.
    pub fn last(&self) -> Option<&Identifier> {
        self.segments.last()
    }

    /// Returns the module path prefix without the trailing function segment.
    pub fn module_segments(&self) -> &[Identifier] {
        self.segments.split_last().map_or(&[], |(_, rest)| rest)
    }
}

#[cfg(test)]
mod tests {
    use super::QualifiedIdentifier;
    use crate::identifier::Identifier;
    use ocelot_base::span::Span;

    #[test]
    fn render_joins_segments_with_double_colons() {
        let identifier = QualifiedIdentifier::new(vec![
            Identifier::new("math", Span::new(0, 4)),
            Identifier::new("greet", Span::new(6, 11)),
            Identifier::new("hello", Span::new(13, 18)),
        ]);

        assert_eq!(identifier.render().as_str(), "math::greet::hello");
    }

    #[test]
    fn span_covers_the_full_identifier() {
        let identifier = QualifiedIdentifier::new(vec![
            Identifier::new("math", Span::new(0, 4)),
            Identifier::new("hello", Span::new(6, 11)),
        ]);

        assert_eq!(identifier.span(), Span::new(0, 11));
    }
}
