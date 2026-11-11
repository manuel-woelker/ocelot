use crate::expression::Expression;

/// An explicit grouping boundary retained for syntax validation and formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParenthesizedExpression {
    /// Grouped expression, without introducing a scope or discarding its value.
    pub expression: Box<Expression>,
}
