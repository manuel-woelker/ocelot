use crate::expression::Expression;

/// An explicit function return, optionally supplying a result expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnStatement {
    /// The returned expression, or no expression for a unit return.
    pub expression: Option<Expression>,
}
