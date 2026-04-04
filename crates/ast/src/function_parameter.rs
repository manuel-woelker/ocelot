use crate::identifier::Identifier;
use crate::type_index::TypeIndex;
use ocelot_base::span::Span;

/// One typed parameter in a user-defined function declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionParameter {
    pub identifier: Identifier,
    pub type_name: Identifier,
    pub ty: TypeIndex,
    pub span: Span,
}

impl FunctionParameter {
    /// Creates one function parameter from its declared name and type.
    pub fn new(identifier: Identifier, type_name: Identifier, span: Span) -> Self {
        Self {
            identifier,
            type_name,
            ty: TypeIndex::unresolved(),
            span,
        }
    }
}
