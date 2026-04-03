use crate::interpreter::Interpreter;
use ocelot_ast::script::Script;
use ocelot_base::result::OcelotResult;
use ocelot_base::source_file::SourceFile;
use ocelot_pal::pal::Pal;

/// Executes a parsed script.
pub fn interpret_script(
    script: &Script,
    source_file: &SourceFile,
    pal: &dyn Pal,
) -> OcelotResult<()> {
    Interpreter::new(pal, source_file).interpret_script(script)
}

#[cfg(test)]
mod tests {
    use super::interpret_script;
    use ocelot_ast::call_expression::CallExpression;
    use ocelot_ast::expression::Expression;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::expression_statement::ExpressionStatement;
    use ocelot_ast::identifier_expression::IdentifierExpression;
    use ocelot_ast::item::Item;
    use ocelot_ast::item_kind::ItemKind;
    use ocelot_ast::script::Script;
    use ocelot_ast::statement::Statement;
    use ocelot_ast::statement_kind::StatementKind;
    use ocelot_ast::string_literal_expression::StringLiteralExpression;
    use ocelot_ast::test_item::TestItem;
    use ocelot_base::error::ErrorKind;
    use ocelot_base::source_file::SourceFile;
    use ocelot_base::span::Span;
    use ocelot_pal::pal_mock::PalMock;

    fn call_expression(name: &str, arguments: Vec<Expression>, span: Span) -> Expression {
        Expression::new(
            ExpressionKind::Call(CallExpression::new(
                Expression::new(
                    ExpressionKind::Identifier(IdentifierExpression::new(name)),
                    Span::new(span.start(), span.start() + name.len()),
                ),
                arguments,
            )),
            span,
        )
    }

    #[test]
    fn interprets_println_string_literal() {
        let script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(Expression::new(
                        ExpressionKind::Call(CallExpression::new(
                            Expression::new(
                                ExpressionKind::Identifier(IdentifierExpression::new("println")),
                                Span::new(0, 7),
                            ),
                            vec![Expression::new(
                                ExpressionKind::StringLiteral(StringLiteralExpression::new(
                                    "hello",
                                )),
                                Span::new(8, 15),
                            )],
                        )),
                        Span::new(0, 16),
                    ))),
                    Span::new(0, 17),
                )),
                Span::new(0, 17),
            )],
            Span::new(0, 17),
        );
        let pal = PalMock::new();
        let source_file = SourceFile::new("examples/hello.ocelot", "println(\"hello\");");

        interpret_script(&script, &source_file, &pal).unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn ignores_test_items_during_normal_script_execution() {
        let script = Script::new(
            vec![
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(Expression::new(
                            ExpressionKind::Call(CallExpression::new(
                                Expression::new(
                                    ExpressionKind::Identifier(IdentifierExpression::new(
                                        "println",
                                    )),
                                    Span::new(0, 7),
                                ),
                                vec![Expression::new(
                                    ExpressionKind::StringLiteral(StringLiteralExpression::new(
                                        "setup",
                                    )),
                                    Span::new(8, 15),
                                )],
                            )),
                            Span::new(0, 16),
                        ))),
                        Span::new(0, 17),
                    )),
                    Span::new(0, 17),
                ),
                Item::new(
                    ItemKind::Test(TestItem::new(
                        "prints hello",
                        vec![Statement::new(
                            StatementKind::Expression(ExpressionStatement::new(Expression::new(
                                ExpressionKind::Call(CallExpression::new(
                                    Expression::new(
                                        ExpressionKind::Identifier(IdentifierExpression::new(
                                            "println",
                                        )),
                                        Span::new(24, 31),
                                    ),
                                    vec![Expression::new(
                                        ExpressionKind::StringLiteral(
                                            StringLiteralExpression::new("hello"),
                                        ),
                                        Span::new(32, 39),
                                    )],
                                )),
                                Span::new(24, 40),
                            ))),
                            Span::new(24, 41),
                        )],
                        Span::new(18, 43),
                    )),
                    Span::new(18, 43),
                ),
            ],
            Span::new(0, 43),
        );
        let pal = PalMock::new();
        let source_file = SourceFile::new(
            "examples/tests.ocelot",
            "println(\"setup\"); test \"prints hello\" { println(\"hello\"); }",
        );

        interpret_script(&script, &source_file, &pal).unwrap();

        assert_eq!(pal.take_printed_output(), "setup\n");
    }

    #[test]
    fn interprets_assert_eq_when_values_match() {
        let script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "assert_eq",
                        vec![
                            Expression::new(
                                ExpressionKind::StringLiteral(StringLiteralExpression::new("same")),
                                Span::new(10, 16),
                            ),
                            Expression::new(
                                ExpressionKind::StringLiteral(StringLiteralExpression::new("same")),
                                Span::new(18, 24),
                            ),
                        ],
                        Span::new(0, 25),
                    ))),
                    Span::new(0, 26),
                )),
                Span::new(0, 26),
            )],
            Span::new(0, 26),
        );
        let source_file = SourceFile::new(
            "examples/assertions.ocelot",
            "assert_eq(\"same\", \"same\");",
        );
        let pal = PalMock::new();

        interpret_script(&script, &source_file, &pal).unwrap();
        assert_eq!(pal.take_printed_output(), "");
    }

    #[test]
    fn reports_assert_eq_mismatches_as_assertion_errors() {
        let script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "assert_eq",
                        vec![
                            Expression::new(
                                ExpressionKind::StringLiteral(StringLiteralExpression::new("a")),
                                Span::new(10, 13),
                            ),
                            Expression::new(
                                ExpressionKind::StringLiteral(StringLiteralExpression::new("b")),
                                Span::new(15, 18),
                            ),
                        ],
                        Span::new(0, 19),
                    ))),
                    Span::new(0, 20),
                )),
                Span::new(0, 20),
            )],
            Span::new(0, 20),
        );
        let source_file = SourceFile::new("examples/assertions.ocelot", "assert_eq(\"a\", \"b\");");
        let pal = PalMock::new();

        let error = interpret_script(&script, &source_file, &pal).unwrap_err();

        assert!(matches!(error.kind(), ErrorKind::AssertionError(_)));
    }

    #[test]
    fn reports_assert_eq_wrong_arity() {
        let script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "assert_eq",
                        vec![Expression::new(
                            ExpressionKind::StringLiteral(StringLiteralExpression::new("only")),
                            Span::new(10, 16),
                        )],
                        Span::new(0, 17),
                    ))),
                    Span::new(0, 18),
                )),
                Span::new(0, 18),
            )],
            Span::new(0, 18),
        );
        let source_file = SourceFile::new("examples/assertions.ocelot", "assert_eq(\"only\");");
        let pal = PalMock::new();

        let error = interpret_script(&script, &source_file, &pal).unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("type error: `assert_eq` expects exactly two arguments")
        );
    }
}
