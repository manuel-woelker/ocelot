use crate::expression::Expression;
use crate::identifier::Identifier;
use crate::type_index::TypeIndex;

/// An immutable binding introduced after evaluating its initializer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetStatement {
    /// Name introduced in the current lexical scope.
    pub identifier: Identifier,
    /// Optional explicitly written binding type and its source span.
    pub type_name: Option<Identifier>,
    /// Expression evaluated once before the name becomes visible.
    pub initializer: Expression,
    /// Inferred or checked type filled in by resolution.
    pub ty: TypeIndex,
}
