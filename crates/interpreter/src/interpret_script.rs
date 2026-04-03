use crate::interpreter::Interpreter;
use ocelot_ast::script::Script;
use ocelot_base::result::OcelotResult;
use ocelot_pal::pal::Pal;

/// Executes a parsed script.
pub fn interpret_script(script: &Script, pal: &dyn Pal) -> OcelotResult<()> {
    Interpreter::new(pal).interpret_script(script)
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
    use ocelot_base::span::Span;
    use ocelot_pal::pal_mock::PalMock;

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

        interpret_script(&script, &pal).unwrap();

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

        interpret_script(&script, &pal).unwrap();

        assert_eq!(pal.take_printed_output(), "setup\n");
    }
}
