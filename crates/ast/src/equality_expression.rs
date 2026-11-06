use crate::equality_operator::EqualityOperator;
use crate::expression::Expression;

/// A comparison of two values of matching types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EqualityExpression {
    /// Operand evaluated first.
    pub left: Box<Expression>,
    /// Comparison to perform.
    pub operator: EqualityOperator,
    /// Operand evaluated after the left completes normally.
    pub right: Box<Expression>,
}
