use crate::expression::Expression;

/// `println(...)` statement node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintlnStatement {
    pub argument: Expression,
}

impl PrintlnStatement {
    /// Creates a println statement from its argument expression.
    pub fn new(argument: Expression) -> Self {
        Self { argument }
    }
}
