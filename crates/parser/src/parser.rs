use crate::lexer::lex::lex;
use crate::lexer::token::Token;
use crate::lexer::token_type::TokenType;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::identifier_expression::IdentifierExpression;
use ocelot_ast::item::Item;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::println_statement::PrintlnStatement;
use ocelot_ast::script::Script;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_ast::string_literal_expression::StringLiteralExpression;
use ocelot_ast::test_item::TestItem;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
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
        let mut items = Vec::new();

        while !self.at(TokenType::EndOfFile) {
            items.push(self.parse_item()?);
        }

        Ok(Script::new(
            items,
            Span::new(0, self.source_file.source().len()),
        ))
    }

    fn parse_item(&mut self) -> OcelotResult<Item> {
        match self.current().token_type {
            TokenType::Test => self.parse_test_item(),
            _ => {
                let statement = self.parse_statement()?;
                let span = statement.span.clone();
                Ok(Item::new(ItemKind::Statement(statement), span))
            }
        }
    }

    fn parse_test_item(&mut self) -> OcelotResult<Item> {
        let test_token = self.expect(TokenType::Test, "expected `test` item")?;
        let name_token = self.expect(TokenType::String, "expected test name string")?;
        let name_literal = self.source_text(&name_token.span);
        let name = SharedString::from(&name_literal[1..name_literal.len() - 1]);
        self.expect(TokenType::LeftBrace, "expected `{` after test name")?;

        let mut body = Vec::new();
        while !self.at(TokenType::RightBrace) {
            if self.at(TokenType::EndOfFile) {
                ocelot_base::bail!("expected `}}` to close test body");
            }
            body.push(self.parse_statement()?);
        }

        let right_brace = self.expect(TokenType::RightBrace, "expected `}` after test body")?;
        let span = Span::new(test_token.span.start(), right_brace.span.end());
        Ok(Item::new(
            ItemKind::Test(TestItem::new(name, body, span.clone())),
            span,
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
        if self.at(TokenType::RightParen) {
            ocelot_base::bail!("type error: `println` expects exactly one argument");
        }
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
