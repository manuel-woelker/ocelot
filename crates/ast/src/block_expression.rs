use crate::statement::Statement;

/// A block whose final semicolon-free expression supplies its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockExpression {
    /// Statements in source order, optionally ending in a result expression.
    pub statements: Vec<Statement>,
}
