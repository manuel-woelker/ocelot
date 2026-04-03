use crate::expression::Expression;

/// Expression statement payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionStatement {
    pub expression: Expression,
}

impl ExpressionStatement {
    /// Creates an expression statement payload.
    pub fn new(expression: Expression) -> Self {
        Self { expression }
    }
}
