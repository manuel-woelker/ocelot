use crate::expression::Expression;

/// Expression statement payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionStatement {
    pub expression: Expression,
    /// Whether this statement discards its value; false only for a block's tail.
    pub has_semicolon: bool,
}

impl ExpressionStatement {
    /// Creates an expression statement payload.
    pub fn new(expression: Expression) -> Self {
        Self {
            expression,
            has_semicolon: true,
        }
    }
}
