use crate::identifier_expression::IdentifierExpression;
use crate::string_literal_expression::StringLiteralExpression;

/// Variants of expressions supported by the current AST scaffold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionKind {
    Identifier(IdentifierExpression),
    StringLiteral(StringLiteralExpression),
}
