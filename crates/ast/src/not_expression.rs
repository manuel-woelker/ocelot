use crate::expression::Expression;

/// Prefix boolean negation expression payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotExpression {
    pub operand: Box<Expression>,
}

impl NotExpression {
    /// Creates a prefix boolean negation expression payload.
    pub fn new(operand: Expression) -> Self {
        Self {
            operand: Box::new(operand),
        }
    }
}
