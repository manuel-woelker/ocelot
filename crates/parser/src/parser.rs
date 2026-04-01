use crate::lexer::lex::lex;
use crate::lexer::token::Token;
use crate::lexer::token_type::TokenType;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::identifier_expression::IdentifierExpression;
use ocelot_ast::println_statement::PrintlnStatement;
use ocelot_ast::script::Script;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_ast::string_literal_expression::StringLiteralExpression;
use ocelot_base::result::OcelotResult;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;

/// Stateful parser context for one source file.
pub struct Parser<'a> {
    source_file: &'a SourceFile,
    tokens: Vec<Token>,
    position: usize,
}

impl<'a> Parser<'a> {
    /// Creates a parser for one source file.
    pub fn new(source_file: &'a SourceFile) -> OcelotResult<Self> {
        let tokens = lex(source_file.source())?.collect();
        Ok(Self {
            source_file,
            tokens,
            position: 0,
        })
    }

    /// Parses the source file into a script AST.
    pub fn parse_script(&mut self) -> OcelotResult<Script> {
        let mut statements = Vec::new();

        while !self.at(TokenType::EndOfFile) {
            statements.push(self.parse_statement()?);
        }

        Ok(Script::new(
            statements,
            Span::new(0, self.source_file.source().len()),
        ))
    }

    fn parse_statement(&mut self) -> OcelotResult<Statement> {
        let start = self.current().span.start();
        let identifier = self.expect(TokenType::Identifier, "expected statement")?;
        let name = self.source_text(&identifier.span);

        if name != "println" {
            ocelot_base::bail!("expected `println` statement");
        }

        self.expect(TokenType::LeftParen, "expected `(` after `println`")?;
        let argument = self.parse_expression()?;
        self.expect(TokenType::RightParen, "expected `)` after argument")?;
        let semicolon = self.expect(TokenType::Semicolon, "expected `;` after statement")?;
        let statement_span = Span::new(start, semicolon.span.end());

        Ok(Statement::new(
            StatementKind::Println(PrintlnStatement::new(argument)),
            statement_span,
        ))
    }

    fn parse_expression(&mut self) -> OcelotResult<Expression> {
        let token = self.current().clone();

        match token.token_type {
            TokenType::String => {
                self.position += 1;
                let literal = self.source_text(&token.span);
                let value = literal[1..literal.len() - 1].to_owned();
                Ok(Expression::new(
                    ExpressionKind::StringLiteral(StringLiteralExpression::new(value)),
                    token.span,
                ))
            }
            TokenType::Identifier => {
                self.position += 1;
                Ok(Expression::new(
                    ExpressionKind::Identifier(IdentifierExpression::new(
                        self.source_text(&token.span),
                    )),
                    token.span,
                ))
            }
            TokenType::Unexpected => {
                ocelot_base::bail!("unexpected token")
            }
            _ => {
                ocelot_base::bail!("expected expression")
            }
        }
    }

    fn expect(&mut self, token_type: TokenType, message: &str) -> OcelotResult<Token> {
        let token = self.current().clone();

        if token.token_type != token_type {
            ocelot_base::bail!("{message}");
        }

        self.position += 1;
        Ok(token)
    }

    fn at(&self, token_type: TokenType) -> bool {
        self.current().token_type == token_type
    }

    fn current(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn source_text(&self, span: &Span) -> &str {
        &self.source_file.source()[span.start()..span.end()]
    }
}
