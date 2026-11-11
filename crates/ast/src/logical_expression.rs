use crate::expression::Expression;
use crate::logical_operator::LogicalOperator;

/// A short-circuit boolean operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalExpression {
    /// Operand evaluated first and exactly once.
    pub left: Box<Expression>,
    /// Determines when the right operand is skipped.
    pub operator: LogicalOperator,
    /// Operand evaluated only when the left does not determine the result.
    pub right: Box<Expression>,
}
