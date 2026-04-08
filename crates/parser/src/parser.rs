use crate::lexer::lex::lex;
use crate::lexer::token::Token;
use crate::lexer::token_type::TokenType;
use ocelot_ast::boolean_literal_expression::BooleanLiteralExpression;
use ocelot_ast::call_expression::CallExpression;
use ocelot_ast::compilation_unit::CompilationUnit;
use ocelot_ast::effect_item::EffectItem;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::expression_statement::ExpressionStatement;
use ocelot_ast::function_effect_clause::FunctionEffectClause;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::function_parameter::FunctionParameter;
use ocelot_ast::identifier::Identifier;
use ocelot_ast::item::Item;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::not_expression::NotExpression;
use ocelot_ast::qualified_identifier::QualifiedIdentifier;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_ast::string_literal_expression::StringLiteralExpression;
use ocelot_ast::template_string_expression::TemplateStringExpression;
use ocelot_ast::template_string_part::TemplateStringPart;
use ocelot_ast::test_item::TestItem;
use ocelot_ast::trivia::Trivia;
use ocelot_ast::trivia_piece::TriviaPiece;
use ocelot_ast::use_item::UseItem;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::error::ErrorKind;
use ocelot_base::error::OcelotError;
use ocelot_base::line_bounds::LineBounds;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;

/// Stateful parser context for one source file.
pub struct Parser<'a> {
    source_file: &'a SourceFile,
    source_diagnostics: &'a mut SourceDiagnostics,
    tokens: Vec<Token>,
    position: usize,
}

impl<'a> Parser<'a> {
    /// Creates a parser for one source file.
    pub fn new(source_file: &'a SourceFile, source_diagnostics: &'a mut SourceDiagnostics) -> Self {
        let tokens = lex(source_file, source_diagnostics);
        Self {
            source_file,
            source_diagnostics,
            tokens,
            position: 0,
        }
    }

    /// Parses the source file into a compilation unit AST.
    pub fn parse_compilation_unit(&mut self) -> OcelotResult<CompilationUnit> {
        if self.source_diagnostics.has_errors() {
            return Err(OcelotError::compilation_error(CompilationStage::Parser));
        }

        let mut items = Vec::new();
        let mut trivia = Trivia::default();

        if self.at(TokenType::EndOfFile) {
            trivia.leading = self.take_current_leading_trivia();
            return Ok(CompilationUnit::with_trivia(
                items,
                trivia,
                Span::new(0, self.source_file.source().len()),
            ));
        }

        trivia.leading = self.take_current_leading_trivia();

        while !self.at(TokenType::EndOfFile) {
            match self.parse_item() {
                Ok(mut item) => {
                    self.attach_same_line_trailing_comments(
                        item.span.end(),
                        &mut item.trivia.trailing,
                    );
                    items.push(item);
                }
                Err(error) if is_parser_compilation_error(&error) => return Err(error),
                Err(error) => return Err(error),
            }
        }

        trivia.trailing = self.take_current_leading_trivia();

        Ok(CompilationUnit::with_trivia(
            items,
            trivia,
            Span::new(0, self.source_file.source().len()),
        ))
    }

    fn parse_item(&mut self) -> OcelotResult<Item> {
        let leading_trivia = self.take_current_leading_trivia();

        let mut item = match self.current().token_type {
            TokenType::Effect => self.parse_effect_item(),
            TokenType::Fun => self.parse_function_item(),
            TokenType::Native => self.parse_function_item(),
            TokenType::Test => self.parse_test_item(),
            TokenType::Use => self.parse_use_item(),
            _ => {
                let statement = self.parse_statement()?;
                let span = statement.span.clone();
                Ok(Item::new(ItemKind::Statement(statement), span))
            }
        }?;

        if item.trivia.is_empty() {
            item.trivia = Trivia::new(leading_trivia, Vec::new());
        }

        Ok(item)
    }

    fn parse_effect_item(&mut self) -> OcelotResult<Item> {
        let effect_token = self.expect(TokenType::Effect, "expected `effect` item")?;
        let name_token = self.expect(TokenType::Identifier, "expected effect name")?;
        let semicolon = self.expect(
            TokenType::Semicolon,
            "expected `;` after effect declaration",
        )?;
        let span = Span::new(effect_token.span.start(), semicolon.span.end());

        Ok(Item::with_trivia(
            ItemKind::Effect(EffectItem::new(
                Identifier::new(self.source_text(&name_token.span), name_token.span),
                span.clone(),
            )),
            Trivia::default(),
            span,
        ))
    }

    fn parse_function_item(&mut self) -> OcelotResult<Item> {
        let native_token = if self.at(TokenType::Native) {
            Some(self.expect(TokenType::Native, "expected `native` keyword")?)
        } else {
            None
        };
        let fun_token = self.expect(TokenType::Fun, "expected `fun` item")?;
        let name_token = self.expect(TokenType::Identifier, "expected function name")?;
        let name = self.source_text(&name_token.span).to_owned();
        let span_start = native_token
            .as_ref()
            .map(|token| token.span.start())
            .unwrap_or_else(|| fun_token.span.start());
        self.expect(TokenType::LeftParen, "expected `(` after function name")?;
        let parameters = self.parse_function_parameters()?;
        self.expect(TokenType::RightParen, "expected `)` after parameter list")?;
        let can_clause = if self.at(TokenType::Can) {
            Some(self.parse_function_effect_clause(TokenType::Can, "expected `can` effect clause")?)
        } else {
            None
        };
        let cannot_clause = if self.at(TokenType::Cannot) {
            Some(self.parse_function_effect_clause(
                TokenType::Cannot,
                "expected `cannot` effect clause",
            )?)
        } else {
            None
        };
        if self.at(TokenType::Can) {
            return self.emit_fatal_diagnostic(
                "function effect clauses must place `can` before `cannot`",
                self.current().span.clone(),
                "`can` must appear first",
            );
        }
        let span;
        let function_item = if native_token.is_some() {
            if self.at(TokenType::LeftBrace) {
                return self.emit_fatal_diagnostic(
                    "native functions must not have a body",
                    self.current().span.clone(),
                    "remove this body",
                );
            }

            let semicolon = self.expect(
                TokenType::Semicolon,
                "expected `;` after native function declaration",
            )?;
            span = Span::new(span_start, semicolon.span.end());
            FunctionItem::new_native(
                Identifier::new(name, name_token.span),
                parameters,
                can_clause,
                cannot_clause,
                span.clone(),
            )
        } else {
            self.expect(TokenType::LeftBrace, "expected `{` before function body")?;

            let body = self.parse_statement_block(
                "expected `}` to close function body",
                "function body ends here",
            )?;

            let right_brace =
                self.expect(TokenType::RightBrace, "expected `}` after function body")?;
            span = Span::new(span_start, right_brace.span.end());
            FunctionItem::new(
                Identifier::new(name, name_token.span),
                parameters,
                can_clause,
                cannot_clause,
                body,
                span.clone(),
            )
        };

        Ok(Item::with_trivia(
            ItemKind::Function(function_item),
            Trivia::default(),
            span,
        ))
    }

    fn parse_function_effect_clause(
        &mut self,
        token_type: TokenType,
        message: &str,
    ) -> OcelotResult<FunctionEffectClause> {
        let keyword = self.expect(token_type, message)?;
        let effect_token = self.expect(TokenType::Identifier, "expected effect name")?;
        let span = Span::new(keyword.span.start(), effect_token.span.end());

        Ok(FunctionEffectClause::new(
            Identifier::new(self.source_text(&effect_token.span), effect_token.span),
            span,
        ))
    }

    fn parse_function_parameters(&mut self) -> OcelotResult<Vec<FunctionParameter>> {
        let mut parameters = Vec::new();

        if self.at(TokenType::RightParen) {
            return Ok(parameters);
        }

        loop {
            let identifier = self.parse_identifier_token("expected parameter name")?;
            self.expect(TokenType::Colon, "expected `:` after parameter name")?;
            let type_name = self.parse_identifier_token("expected parameter type")?;
            let span = Span::new(identifier.span.start(), type_name.span.end());
            parameters.push(FunctionParameter::new(identifier, type_name, span));

            if !self.at(TokenType::Comma) {
                break;
            }

            self.position += 1;

            if self.at(TokenType::RightParen) {
                return self.emit_fatal_diagnostic(
                    "expected parameter after `,`",
                    self.current().span.clone(),
                    "parameter expected here",
                );
            }
        }

        Ok(parameters)
    }

    fn parse_test_item(&mut self) -> OcelotResult<Item> {
        let test_token = self.expect(TokenType::Test, "expected `test` item")?;
        if self.at(TokenType::StringStart) {
            return self.emit_fatal_diagnostic(
                "test names must be plain string literals",
                self.current().span.clone(),
                "interpolation is not allowed here",
            );
        }
        let name_token = self.expect(TokenType::String, "expected test name string")?;
        let name_literal = self.source_text(&name_token.span);
        let name = SharedString::from(&name_literal[1..name_literal.len() - 1]);
        self.expect(TokenType::LeftBrace, "expected `{` after test name")?;

        let body =
            self.parse_statement_block("expected `}` to close test body", "test body ends here")?;

        let right_brace = self.expect(TokenType::RightBrace, "expected `}` after test body")?;
        let span = Span::new(test_token.span.start(), right_brace.span.end());
        Ok(Item::with_trivia(
            ItemKind::Test(TestItem::new(name, body, span.clone())),
            Trivia::default(),
            span,
        ))
    }

    fn parse_use_item(&mut self) -> OcelotResult<Item> {
        let use_token = self.expect(TokenType::Use, "expected `use` item")?;
        let mut module_segments = vec![self.parse_identifier_token("expected module name")?];

        self.expect(
            TokenType::DoubleColon,
            "expected `::` after imported module path",
        )?;

        let (imported_names, end_span) = if self.at(TokenType::LeftBrace) {
            self.parse_grouped_import_names()?
        } else {
            let final_segment = self.parse_identifier_token("expected imported name")?;

            if !self.at(TokenType::DoubleColon) {
                (vec![final_segment.clone()], final_segment.span.clone())
            } else {
                module_segments.push(final_segment);

                loop {
                    self.expect(
                        TokenType::DoubleColon,
                        "expected `::` after imported module path segment",
                    )?;

                    if self.at(TokenType::LeftBrace) {
                        let (imported_names, end_span) = self.parse_grouped_import_names()?;
                        break (imported_names, end_span);
                    }

                    let segment = self
                        .parse_identifier_token("expected identifier after `::` in `use` item")?;
                    if !self.at(TokenType::DoubleColon) {
                        break (vec![segment.clone()], segment.span.clone());
                    }

                    module_segments.push(segment);
                }
            }
        };

        let semicolon = self.expect(TokenType::Semicolon, "expected `;` after `use` item")?;
        let span = Span::new(use_token.span.start(), semicolon.span.end());
        let module_path = QualifiedIdentifier::new(module_segments.clone());

        if module_segments.is_empty() {
            return self.emit_fatal_diagnostic(
                "expected module path in `use` item",
                end_span,
                "module path expected here",
            );
        }

        Ok(Item::with_trivia(
            ItemKind::Use(UseItem::new(module_path, imported_names, span.clone())),
            Trivia::default(),
            span,
        ))
    }

    fn parse_statement(&mut self) -> OcelotResult<Statement> {
        let leading_trivia = self.take_current_leading_trivia();
        let expression = self.parse_expression()?;
        let semicolon = self.expect(TokenType::Semicolon, "expected `;` after statement")?;
        let statement_span = Span::new(expression.span.start(), semicolon.span.end());

        Ok(Statement::with_trivia(
            StatementKind::Expression(ExpressionStatement::new(expression)),
            Trivia::new(leading_trivia, Vec::new()),
            statement_span,
        ))
    }

    fn parse_expression(&mut self) -> OcelotResult<Expression> {
        self.parse_prefix_expression()
    }

    fn parse_prefix_expression(&mut self) -> OcelotResult<Expression> {
        self.reject_comments_before_current_token()?;

        if self.at(TokenType::Not) {
            let not_token = self.expect(TokenType::Not, "expected `not` operator")?;
            let operand = self.parse_prefix_expression()?;
            let span = Span::new(not_token.span.start(), operand.span.end());

            return Ok(Expression::new(
                ExpressionKind::Not(NotExpression::new(operand)),
                span,
            ));
        }

        self.parse_call_expression()
    }

    fn parse_call_expression(&mut self) -> OcelotResult<Expression> {
        let mut expression = self.parse_primary_expression()?;

        loop {
            self.reject_comments_before_current_token()?;

            if !self.at(TokenType::LeftParen) {
                break;
            }

            expression = self.parse_call_expression_suffix(expression)?;
        }

        Ok(expression)
    }

    fn parse_primary_expression(&mut self) -> OcelotResult<Expression> {
        self.reject_comments_before_current_token()?;
        let token = self.current().clone();

        match token.token_type {
            TokenType::False => {
                self.position += 1;
                Ok(Expression::new(
                    ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(false)),
                    token.span,
                ))
            }
            TokenType::String => {
                self.position += 1;
                let literal = self.source_text(&token.span);
                let value = literal[1..literal.len() - 1].to_owned();
                Ok(Expression::new(
                    ExpressionKind::StringLiteral(StringLiteralExpression::new(value)),
                    token.span,
                ))
            }
            TokenType::StringStart => self.parse_template_string_expression(),
            TokenType::True => {
                self.position += 1;
                Ok(Expression::new(
                    ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true)),
                    token.span,
                ))
            }
            TokenType::Identifier => self.parse_identifier_expression(),
            TokenType::Not => self.emit_fatal_diagnostic(
                "expected expression after `not`",
                token.span,
                "operand expected here",
            ),
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

    fn parse_identifier_expression(&mut self) -> OcelotResult<Expression> {
        let mut segments = vec![self.parse_identifier_token("expected identifier")?];

        while self.at(TokenType::DoubleColon) {
            self.position += 1;
            segments.push(self.parse_identifier_token("expected identifier after `::`")?);
        }

        if segments.len() == 1 {
            let identifier = segments.pop().expect("single identifier should exist");
            let span = identifier.span.clone();
            return Ok(Expression::new(
                ExpressionKind::Identifier(identifier),
                span,
            ));
        }

        let qualified_identifier = QualifiedIdentifier::new(segments);
        let span = qualified_identifier.span();
        Ok(Expression::new(
            ExpressionKind::QualifiedIdentifier(qualified_identifier),
            span,
        ))
    }

    fn parse_call_expression_suffix(&mut self, callee: Expression) -> OcelotResult<Expression> {
        self.expect(TokenType::LeftParen, "expected `(` after callee")?;
        let mut arguments = Vec::new();

        if !self.at(TokenType::RightParen) {
            loop {
                arguments.push(self.parse_expression()?);

                if !self.at(TokenType::Comma) {
                    break;
                }

                self.position += 1;
            }
        }

        let right_paren = self.expect(TokenType::RightParen, "expected `)` after argument list")?;
        let span = Span::new(callee.span.start(), right_paren.span.end());
        Ok(Expression::new(
            ExpressionKind::Call(CallExpression::new(callee, arguments)),
            span,
        ))
    }

    fn parse_template_string_expression(&mut self) -> OcelotResult<Expression> {
        let string_start = self.expect(TokenType::StringStart, "expected `\"` to start string")?;
        let mut parts = Vec::new();

        loop {
            self.reject_comments_before_current_token()?;

            if self.at(TokenType::StringText) {
                let token = self.expect(TokenType::StringText, "expected string text")?;
                parts.push(TemplateStringPart::Text(
                    self.source_text(&token.span).into(),
                ));
                continue;
            }

            if self.at(TokenType::InterpolationStart) {
                self.expect(
                    TokenType::InterpolationStart,
                    "expected `${` to start interpolation",
                )?;

                if self.at(TokenType::RightBrace) {
                    return self.emit_fatal_diagnostic(
                        "expected expression inside template interpolation",
                        self.current().span.clone(),
                        "expression expected here",
                    );
                }

                let expression = self.parse_expression()?;
                self.expect(TokenType::RightBrace, "expected `}` after interpolation")?;
                parts.push(TemplateStringPart::Interpolation(expression));
                continue;
            }

            let string_end = self.expect(TokenType::StringEnd, "expected `\"` after string")?;
            let span = Span::new(string_start.span.start(), string_end.span.end());
            return Ok(Expression::new(
                ExpressionKind::TemplateString(TemplateStringExpression::new(parts)),
                span,
            ));
        }
    }

    fn parse_grouped_import_names(&mut self) -> OcelotResult<(Vec<Identifier>, Span)> {
        self.expect(
            TokenType::LeftBrace,
            "expected `{` before grouped import names",
        )?;
        let mut imported_names = Vec::new();

        loop {
            imported_names.push(self.parse_identifier_token("expected imported name inside `{}`")?);

            if !self.at(TokenType::Comma) {
                break;
            }

            self.position += 1;
        }

        let right_brace =
            self.expect(TokenType::RightBrace, "expected `}` after grouped imports")?;
        Ok((imported_names, right_brace.span))
    }

    fn parse_identifier_token(&mut self, message: &str) -> OcelotResult<Identifier> {
        let token = self.expect(TokenType::Identifier, message)?;
        Ok(Identifier::new(
            self.source_text(&token.span),
            token.span.clone(),
        ))
    }

    fn expect(&mut self, token_type: TokenType, message: &str) -> OcelotResult<Token> {
        self.reject_comments_before_current_token()?;
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

    fn current_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.position]
    }

    fn take_current_leading_trivia(&mut self) -> Vec<TriviaPiece> {
        std::mem::take(&mut self.current_mut().leading_trivia.leading)
    }

    fn attach_same_line_trailing_comments(
        &mut self,
        node_end: usize,
        trailing: &mut Vec<TriviaPiece>,
    ) {
        let source = self.source_file.source();
        let node_line = LineBounds::new(source, node_end).line_number;
        let mut split_index = 0;

        while split_index < self.current().leading_trivia.leading.len() {
            let piece = &self.current().leading_trivia.leading[split_index];

            if matches!(piece, TriviaPiece::Newlines { .. }) {
                break;
            }

            if !piece.is_comment() {
                break;
            }

            if LineBounds::new(source, piece.span().start()).line_number != node_line {
                break;
            }

            split_index += 1;
        }

        let leading = &mut self.current_mut().leading_trivia.leading;
        trailing.extend(leading.drain(0..split_index));
    }

    fn parse_statement_block(
        &mut self,
        eof_message: &str,
        eof_annotation: &str,
    ) -> OcelotResult<Vec<Statement>> {
        let mut body = Vec::new();

        while !self.at(TokenType::RightBrace) {
            if self.at(TokenType::EndOfFile) {
                return self.emit_fatal_diagnostic(
                    eof_message,
                    self.current().span.clone(),
                    eof_annotation,
                );
            }

            let mut statement = self.parse_statement()?;
            self.attach_same_line_trailing_comments(
                statement.span.end(),
                &mut statement.trivia.trailing,
            );
            body.push(statement);
        }

        Ok(body)
    }

    fn reject_comments_before_current_token(&mut self) -> OcelotResult<()> {
        let comment_span = self
            .current()
            .leading_trivia
            .leading
            .iter()
            .find(|piece| piece.is_comment())
            .map(|piece| piece.span().clone());

        if let Some(span) = comment_span {
            return self.emit_fatal_diagnostic(
                "comments are only allowed before items or statements",
                span,
                "comment is not allowed here",
            );
        }

        Ok(())
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
        self.source_diagnostics
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
        let line_bounds = LineBounds::new(self.source_file.source(), span.start());
        let source_line = &self.source_file.source()[line_bounds.line_start..line_bounds.line_end];
        let relative_start = span.start().saturating_sub(line_bounds.line_start);
        let relative_end = span.end().saturating_sub(line_bounds.line_start);

        SourceDiagnostic::new(DiagnosticLevel::Error, &self.source_file.path, message).with_excerpt(
            SourceExcerpt::new(&self.source_file.path, line_bounds.line_number, source_line)
                .with_annotation(SourceAnnotation::new(
                    Span::new(relative_start, relative_end),
                    annotation,
                )),
        )
    }
}

fn is_parser_compilation_error(error: &OcelotError) -> bool {
    matches!(
        error.kind(),
        ErrorKind::CompilationError(CompilationStage::Parser)
    )
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::expression_statement::ExpressionStatement;
    use ocelot_ast::item_kind::ItemKind;
    use ocelot_ast::statement_kind::StatementKind;
    use ocelot_ast::use_item::UseItem;
    use ocelot_base::source_diagnostics::SourceDiagnostics;
    use ocelot_base::source_file::SourceFile;

    #[test]
    fn parses_qualified_call_targets() {
        let source_file = SourceFile::new("examples/module.ocelot", "math::greet::hello();");
        let mut source_diagnostics = SourceDiagnostics::default();
        let mut parser = Parser::new(&source_file, &mut source_diagnostics);

        let script = parser.parse_compilation_unit().unwrap();
        let ItemKind::Statement(statement) = &script.items[0].kind else {
            panic!("expected statement");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
        let ExpressionKind::Call(call_expression) = &expression.kind else {
            panic!("expected call expression");
        };
        let ExpressionKind::QualifiedIdentifier(qualified_identifier) =
            &call_expression.callee.kind
        else {
            panic!("expected qualified identifier");
        };

        assert_eq!(qualified_identifier.render().as_str(), "math::greet::hello");
    }

    #[test]
    fn parses_single_name_use_items() {
        let source_file = SourceFile::new("examples/module.ocelot-script", "use helper::greet;");
        let mut source_diagnostics = SourceDiagnostics::default();
        let mut parser = Parser::new(&source_file, &mut source_diagnostics);

        let script = parser.parse_compilation_unit().unwrap();
        let ItemKind::Use(UseItem {
            module_path,
            imported_names,
            ..
        }) = &script.items[0].kind
        else {
            panic!("expected use item");
        };

        assert_eq!(module_path.render().as_str(), "helper");
        assert_eq!(imported_names.len(), 1);
        assert_eq!(imported_names[0].name.as_str(), "greet");
    }

    #[test]
    fn parses_grouped_use_items() {
        let source_file = SourceFile::new(
            "examples/module.ocelot-script",
            "use math::trig::{sin, cos};",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let mut parser = Parser::new(&source_file, &mut source_diagnostics);

        let script = parser.parse_compilation_unit().unwrap();
        let ItemKind::Use(UseItem {
            module_path,
            imported_names,
            ..
        }) = &script.items[0].kind
        else {
            panic!("expected use item");
        };

        assert_eq!(module_path.render().as_str(), "math::trig");
        assert_eq!(
            imported_names
                .iter()
                .map(|identifier| identifier.name.as_str())
                .collect::<Vec<_>>(),
            vec!["sin", "cos"]
        );
    }
}
