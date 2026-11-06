use crate::expression::Expression;

/// A conditional expression selecting one branch by a boolean condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfExpression {
    /// Condition evaluated once before either branch.
    pub condition: Box<Expression>,
    /// Block evaluated when the condition is true.
    pub then_branch: Box<Expression>,
    /// Else block or nested conditional; omission supplies unit.
    pub else_branch: Option<Box<Expression>>,
}
