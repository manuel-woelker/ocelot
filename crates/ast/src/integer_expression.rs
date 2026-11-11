use crate::expression::Expression;
use crate::integer_operator::IntegerOperator;

/// Integer arithmetic or ordering, with left-to-right evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerExpression {
    /// First evaluated operand.
    pub left: Box<Expression>,
    /// Checked arithmetic or ordering operation.
    pub operator: IntegerOperator,
    /// Second evaluated operand.
    pub right: Box<Expression>,
}
