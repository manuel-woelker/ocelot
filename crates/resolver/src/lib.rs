//! Name resolution for `ocelot`.

use ocelot_ast::call_expression::CallExpression;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::expression_statement::ExpressionStatement;
use ocelot_ast::item::Item;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::program_environment::ProgramEnvironment;
use ocelot_ast::script::Script;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_ast::test_item::TestItem;
use ocelot_base::compilation_context::CompilationContext;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::error::OcelotError;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;

/// Resolves names within a parsed program.
pub fn resolve(
    script: &mut Script,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    environment: &ProgramEnvironment,
) -> OcelotResult<()> {
    Resolver::new(source_file, compilation_context, environment).resolve_script(script)
}

struct Resolver<'a> {
    source_file: &'a SourceFile,
    compilation_context: &'a mut CompilationContext,
    environment: &'a ProgramEnvironment,
}

impl<'a> Resolver<'a> {
    fn new(
        source_file: &'a SourceFile,
        compilation_context: &'a mut CompilationContext,
        environment: &'a ProgramEnvironment,
    ) -> Self {
        Self {
            source_file,
            compilation_context,
            environment,
        }
    }

    fn resolve_script(&mut self, script: &mut Script) -> OcelotResult<()> {
        for item in &mut script.items {
            self.resolve_item(item);
        }

        if self.compilation_context.has_errors() {
            return Err(
                OcelotError::compilation_error(CompilationStage::Resolver).with_source(
                    OcelotError::message(render_source_diagnostics(
                        &self.compilation_context.source_diagnostics.diagnostics,
                    )),
                ),
            );
        }

        Ok(())
    }

    fn resolve_item(&mut self, item: &mut Item) {
        match &mut item.kind {
            ItemKind::Statement(statement) => self.resolve_statement(statement),
            ItemKind::Test(test_item) => self.resolve_test_item(test_item),
        }
    }

    fn resolve_test_item(&mut self, test_item: &mut TestItem) {
        for statement in &mut test_item.body {
            self.resolve_statement(statement);
        }
    }

    fn resolve_statement(&mut self, statement: &mut Statement) {
        match &mut statement.kind {
            StatementKind::Expression(ExpressionStatement { expression }) => {
                self.resolve_expression(expression);
            }
        }
    }

    fn resolve_expression(&mut self, expression: &mut Expression) {
        match &mut expression.kind {
            ExpressionKind::BooleanLiteral(_)
            | ExpressionKind::Identifier(_)
            | ExpressionKind::StringLiteral(_) => {}
            ExpressionKind::Not(not_expression) => {
                self.resolve_expression(&mut not_expression.operand);
            }
            ExpressionKind::Call(call_expression) => self.resolve_call_expression(call_expression),
        }
    }

    fn resolve_call_expression(&mut self, call_expression: &mut CallExpression) {
        self.resolve_expression(&mut call_expression.callee);
        for argument in &mut call_expression.arguments {
            self.resolve_expression(argument);
        }

        let ExpressionKind::Identifier(identifier) = &call_expression.callee.kind else {
            self.add_diagnostic(
                "only identifier calls are supported",
                call_expression.callee.span.clone(),
                "callee must be an identifier",
            );
            return;
        };

        let Some(function_index) = self.environment.resolve_function(&identifier.name) else {
            self.add_diagnostic(
                format!("unknown function `{}`", identifier.name),
                call_expression.callee.span.clone(),
                "unknown function",
            );
            return;
        };

        call_expression.resolve_to(function_index);
    }

    fn add_diagnostic(
        &mut self,
        message: impl Into<SharedString>,
        span: Span,
        annotation: impl Into<SharedString>,
    ) {
        self.compilation_context
            .add_diagnostic(self.source_diagnostic(message, span, annotation));
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

#[cfg(test)]
mod tests {
    use super::resolve;
    use ocelot_ast::call_expression::CallExpression;
    use ocelot_ast::expression::Expression;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::expression_statement::ExpressionStatement;
    use ocelot_ast::function_definition::FunctionDefinition;
    use ocelot_ast::identifier_expression::IdentifierExpression;
    use ocelot_ast::item::Item;
    use ocelot_ast::item_kind::ItemKind;
    use ocelot_ast::native_function::NativeFunction;
    use ocelot_ast::program_environment::ProgramEnvironment;
    use ocelot_ast::script::Script;
    use ocelot_ast::statement::Statement;
    use ocelot_ast::statement_kind::StatementKind;
    use ocelot_ast::string_literal_expression::StringLiteralExpression;
    use ocelot_ast::test_item::TestItem;
    use ocelot_base::compilation_context::CompilationContext;
    use ocelot_base::compilation_stage::CompilationStage;
    use ocelot_base::source_file::SourceFile;
    use ocelot_base::span::Span;

    fn identifier(name: &str, span: Span) -> Expression {
        Expression::new(
            ExpressionKind::Identifier(IdentifierExpression::new(name)),
            span,
        )
    }

    fn string_literal(value: &str, span: Span) -> Expression {
        Expression::new(
            ExpressionKind::StringLiteral(StringLiteralExpression::new(value)),
            span,
        )
    }

    fn call(callee: Expression, arguments: Vec<Expression>, span: Span) -> Expression {
        Expression::new(
            ExpressionKind::Call(CallExpression::new(callee, arguments)),
            span,
        )
    }

    fn test_program_environment() -> ProgramEnvironment {
        ProgramEnvironment::new(vec![
            FunctionDefinition::new("println", NativeFunction::Println),
            FunctionDefinition::new("assert", NativeFunction::Assert),
            FunctionDefinition::new("assert_eq", NativeFunction::AssertEq),
        ])
    }

    #[test]
    fn resolves_native_call_expressions() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("println", Span::new(0, 7)),
                        vec![string_literal("hello", Span::new(8, 15))],
                        Span::new(0, 16),
                    ))),
                    Span::new(0, 17),
                )),
                Span::new(0, 17),
            )],
            Span::new(0, 17),
        );
        let source_file = SourceFile::new("examples/hello.ocelot", "println(\"hello\");");
        let environment = test_program_environment();
        let println_index = environment.resolve_function("println").unwrap();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &environment).unwrap();

        let ItemKind::Statement(statement) = &script.items[0].kind else {
            panic!("expected statement");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
        let ExpressionKind::Call(call_expression) = &expression.kind else {
            panic!("expected call expression");
        };
        assert_eq!(call_expression.function_index().unwrap(), println_index);
    }

    #[test]
    fn resolves_calls_inside_test_items() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Test(TestItem::new(
                    "prints",
                    vec![Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("assert", Span::new(17, 23)),
                            vec![identifier("true", Span::new(24, 28))],
                            Span::new(17, 29),
                        ))),
                        Span::new(17, 30),
                    )],
                    Span::new(0, 32),
                )),
                Span::new(0, 32),
            )],
            Span::new(0, 32),
        );
        let source_file =
            SourceFile::new("examples/tests.ocelot", "test \"prints\" { assert(true); }");
        let environment = test_program_environment();
        let assert_index = environment.resolve_function("assert").unwrap();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &environment).unwrap();

        let ItemKind::Test(test_item) = &script.items[0].kind else {
            panic!("expected test item");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &test_item.body[0].kind;
        let ExpressionKind::Call(call_expression) = &expression.kind else {
            panic!("expected call expression");
        };
        assert_eq!(call_expression.function_index().unwrap(), assert_index);
    }

    #[test]
    fn resolves_nested_calls_recursively() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("println", Span::new(0, 7)),
                        vec![call(
                            identifier("println", Span::new(8, 15)),
                            vec![string_literal("hello", Span::new(16, 23))],
                            Span::new(8, 24),
                        )],
                        Span::new(0, 25),
                    ))),
                    Span::new(0, 26),
                )),
                Span::new(0, 26),
            )],
            Span::new(0, 26),
        );
        let source_file = SourceFile::new("examples/nested.ocelot", "println(println(\"hello\"));");
        let environment = test_program_environment();
        let println_index = environment.resolve_function("println").unwrap();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &environment).unwrap();

        let ItemKind::Statement(statement) = &script.items[0].kind else {
            panic!("expected statement");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
        let ExpressionKind::Call(outer_call) = &expression.kind else {
            panic!("expected outer call");
        };
        assert_eq!(outer_call.function_index().unwrap(), println_index);

        let ExpressionKind::Call(inner_call) = &outer_call.arguments[0].kind else {
            panic!("expected nested call");
        };
        assert_eq!(inner_call.function_index().unwrap(), println_index);
    }

    #[test]
    fn reports_unknown_function_calls_as_resolver_errors() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("printline", Span::new(0, 9)),
                        vec![string_literal("hello", Span::new(10, 17))],
                        Span::new(0, 18),
                    ))),
                    Span::new(0, 19),
                )),
                Span::new(0, 19),
            )],
            Span::new(0, 19),
        );
        let source_file = SourceFile::new("examples/broken.ocelot", "printline(\"hello\");");
        let environment = test_program_environment();
        let mut context = CompilationContext::default();

        let error = resolve(&mut script, &source_file, &mut context, &environment).unwrap_err();

        assert!(matches!(
            error.kind(),
            ocelot_base::error::ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(
            error
                .to_test_string()
                .contains("unknown function `printline`")
        );
        assert!(error.to_test_string().contains("printline(\"hello\");"));
        assert!(
            error
                .to_test_string()
                .contains("at examples/broken.ocelot:1:1")
        );
    }

    #[test]
    fn reports_non_identifier_callees_as_resolver_errors() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        string_literal("hello", Span::new(0, 7)),
                        Vec::new(),
                        Span::new(0, 9),
                    ))),
                    Span::new(0, 10),
                )),
                Span::new(0, 10),
            )],
            Span::new(0, 10),
        );
        let source_file = SourceFile::new("examples/broken.ocelot", "\"hello\"();");
        let environment = test_program_environment();
        let mut context = CompilationContext::default();

        let error = resolve(&mut script, &source_file, &mut context, &environment).unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("only identifier calls are supported")
        );
    }
}
