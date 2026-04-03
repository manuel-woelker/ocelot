use crate::expression_statement::ExpressionStatement;

/// Variants of top-level script statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementKind {
    Expression(ExpressionStatement),
}
