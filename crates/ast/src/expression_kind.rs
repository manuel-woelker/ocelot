use crate::block_expression::BlockExpression;
use crate::boolean_literal_expression::BooleanLiteralExpression;
use crate::call_expression::CallExpression;
use crate::identifier::Identifier;
use crate::if_expression::IfExpression;
use crate::not_expression::NotExpression;
use crate::qualified_identifier::QualifiedIdentifier;
use crate::string_literal_expression::StringLiteralExpression;
use crate::template_string_expression::TemplateStringExpression;

/// Variants of expressions supported by the current AST scaffold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionKind {
    Block(BlockExpression),
    BooleanLiteral(BooleanLiteralExpression),
    Call(CallExpression),
    Identifier(Identifier),
    If(IfExpression),
    Not(NotExpression),
    QualifiedIdentifier(QualifiedIdentifier),
    StringLiteral(StringLiteralExpression),
    TemplateString(TemplateStringExpression),
}
