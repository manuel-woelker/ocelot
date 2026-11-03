use crate::expression_statement::ExpressionStatement;
use crate::return_statement::ReturnStatement;

/// Statement variants used by scripts, tests, and function bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementKind {
    Expression(ExpressionStatement),
    Return(ReturnStatement),
}
