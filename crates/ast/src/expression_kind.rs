use crate::boolean_literal_expression::BooleanLiteralExpression;
use crate::call_expression::CallExpression;
use crate::identifier::Identifier;
use crate::not_expression::NotExpression;
use crate::string_literal_expression::StringLiteralExpression;

/// Variants of expressions supported by the current AST scaffold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionKind {
    BooleanLiteral(BooleanLiteralExpression),
    Call(CallExpression),
    Identifier(Identifier),
    Not(NotExpression),
    StringLiteral(StringLiteralExpression),
}
