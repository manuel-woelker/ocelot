use crate::expression::Expression;

/// Function call expression payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallExpression {
    pub callee: Box<Expression>,
    pub arguments: Vec<Expression>,
}

impl CallExpression {
    /// Creates a call expression payload.
    pub fn new(callee: Expression, arguments: Vec<Expression>) -> Self {
        Self {
            callee: Box::new(callee),
            arguments,
        }
    }
}
