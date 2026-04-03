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
use ocelot_base::compilation_context::CompilationContext;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::error::ErrorKind;
use ocelot_base::error::OcelotError;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;

/// Stateful parser context for one source file.
pub struct Parser<'a> {
    source_file: &'a SourceFile,
    compilation_context: &'a mut CompilationContext,
    tokens: Vec<Token>,
    position: usize,
}

impl<'a> Parser<'a> {
    /// Creates a parser for one source file.
    pub fn new(
        source_file: &'a SourceFile,
        compilation_context: &'a mut CompilationContext,
    ) -> Self {
        let tokens = lex(source_file, compilation_context);
        Self {
            source_file,
            compilation_context,
            tokens,
            position: 0,
        }
    }

    /// Parses the source file into a script AST.
    pub fn parse_script(&mut self) -> OcelotResult<Script> {
        if self.compilation_context.has_errors() {
            return Err(OcelotError::compilation_error(CompilationStage::Parser));
        }

        let mut items = Vec::new();

        while !self.at(TokenType::EndOfFile) {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(error) if is_parser_compilation_error(&error) => return Err(error),
                Err(error) => return Err(error),
            }
        }

        Ok(Script::new(
            items,
            Span::new(0, self.source_file.source().len()),
        ))
    }

    fn parse_item(&mut self) -> OcelotResult<Item> {
        match self.current().token_type {
            TokenType::Test => self.parse_test_item(),
            _ => Ok({
                let statement = self.parse_statement()?;
                let span = statement.span.clone();
                Item::new(ItemKind::Statement(statement), span)
            }),
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
                return self.emit_fatal_diagnostic(
                    "expected `}` to close test body",
                    self.current().span.clone(),
                    "test body ends here",
                );
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
            return self.emit_fatal_diagnostic(
                "expected `println` statement",
                identifier.span,
                "statement is not supported",
            );
        }

        self.expect(TokenType::LeftParen, "expected `(` after `println`")?;
        if self.at(TokenType::RightParen) {
            return self.emit_fatal_diagnostic(
                "type error: `println` expects exactly one argument",
                self.current().span.clone(),
                "missing argument",
            );
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
                self.emit_fatal_diagnostic("unexpected token", token.span, "unexpected character")
            }
            _ => self.emit_fatal_diagnostic(
                "expected expression",
                token.span,
                "expression expected here",
            ),
        }
    }

    fn expect(&mut self, token_type: TokenType, message: &str) -> OcelotResult<Token> {
        let token = self.current().clone();

        if token.token_type != token_type {
            return self.emit_fatal_diagnostic(message, token.span, "found here");
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

    fn emit_fatal_diagnostic<T>(
        &mut self,
        message: impl Into<SharedString>,
        span: Span,
        annotation: impl Into<SharedString>,
    ) -> OcelotResult<T> {
        self.compilation_context
            .add_diagnostic(self.source_diagnostic(message, span, annotation));
        Err(OcelotError::compilation_error(CompilationStage::Parser))
    }

    fn source_diagnostic(
        &self,
        message: impl Into<SharedString>,
        span: Span,
        annotation: impl Into<SharedString>,
    ) -> SourceDiagnostic {
        let message = message.into();
        let annotation = annotation.into();
        let (line_number, line_start, line_end) = self.line_bounds(span.start());
        let source_line = &self.source_file.source()[line_start..line_end];
        let relative_start = span.start().saturating_sub(line_start);
        let relative_end = span.end().saturating_sub(line_start);

        SourceDiagnostic::new(DiagnosticLevel::Error, &self.source_file.path, message).with_excerpt(
            SourceExcerpt::new(&self.source_file.path, line_number, source_line).with_annotation(
                SourceAnnotation::new(Span::new(relative_start, relative_end), annotation),
            ),
        )
    }

    fn line_bounds(&self, index: usize) -> (usize, usize, usize) {
        let source = self.source_file.source();
        let line_start = source[..index].rfind('\n').map_or(0, |offset| offset + 1);
        let line_end = source[index..]
            .find('\n')
            .map_or(source.len(), |offset| index + offset);
        let line_number = source[..line_start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;

        (line_number, line_start, line_end)
    }
}

fn is_parser_compilation_error(error: &OcelotError) -> bool {
    matches!(
        error.kind(),
        ErrorKind::CompilationError(CompilationStage::Parser)
    )
}
