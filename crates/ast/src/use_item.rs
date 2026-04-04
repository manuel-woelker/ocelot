use crate::identifier::Identifier;
use crate::qualified_identifier::QualifiedIdentifier;
use ocelot_base::span::Span;

/// Top-level `use` declaration importing functions from one module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseItem {
    pub module_path: QualifiedIdentifier,
    pub imported_names: Vec<Identifier>,
    pub span: Span,
}

impl UseItem {
    /// Creates a `use` item from its module path, imported names, and source span.
    pub fn new(
        module_path: QualifiedIdentifier,
        imported_names: Vec<Identifier>,
        span: Span,
    ) -> Self {
        Self {
            module_path,
            imported_names,
            span,
        }
    }
}
