use crate::expression::Expression;

/// Checked unary integer negation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegateExpression {
    /// Negated expression; a direct literal may contain the minimum-int magnitude.
    pub operand: Box<Expression>,
}
