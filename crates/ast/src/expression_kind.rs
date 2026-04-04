use crate::boolean_literal_expression::BooleanLiteralExpression;
use crate::call_expression::CallExpression;
use crate::identifier_expression::IdentifierExpression;
use crate::not_expression::NotExpression;
use crate::string_literal_expression::StringLiteralExpression;

/// Variants of expressions supported by the current AST scaffold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionKind {
    BooleanLiteral(BooleanLiteralExpression),
    Call(CallExpression),
    Identifier(IdentifierExpression),
    Not(NotExpression),
    StringLiteral(StringLiteralExpression),
}
