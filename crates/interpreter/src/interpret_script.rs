use crate::interpreter::Interpreter;
use ocelot_ast::compilation_unit::CompilationUnit;
use ocelot_base::result::OcelotResult;
use ocelot_base::source_file::SourceFile;
use ocelot_pal::pal::Pal;
use ocelot_semantic::symbol_table::SymbolTable;

/// Executes a parsed script.
pub fn interpret_script(
    script: &CompilationUnit,
    source_file: &SourceFile,
    environment: &SymbolTable,
    pal: &dyn Pal,
) -> OcelotResult<()> {
    Interpreter::new(pal, source_file, environment).interpret_script(script)
}

#[cfg(test)]
mod tests {
    use super::interpret_script as interpret_resolved_script;
    use ocelot_ast::boolean_literal_expression::BooleanLiteralExpression;
    use ocelot_ast::call_expression::CallExpression;
    use ocelot_ast::compilation_unit::CompilationUnit;
    use ocelot_ast::expression::Expression;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::expression_statement::ExpressionStatement;
    use ocelot_ast::function_item::FunctionItem;
    use ocelot_ast::function_parameter::FunctionParameter;
    use ocelot_ast::identifier::Identifier;
    use ocelot_ast::item::Item;
    use ocelot_ast::item_kind::ItemKind;
    use ocelot_ast::not_expression::NotExpression;
    use ocelot_ast::statement::Statement;
    use ocelot_ast::statement_kind::StatementKind;
    use ocelot_ast::string_literal_expression::StringLiteralExpression;
    use ocelot_ast::test_item::TestItem;
    use ocelot_base::error::ErrorKind;
    use ocelot_base::result::OcelotResult;
    use ocelot_base::source_file::SourceFile;
    use ocelot_base::span::Span;
    use ocelot_pal::pal_mock::PalMock;
    use ocelot_resolver::resolution::resolve;
    use ocelot_semantic::compilation_context::CompilationContext;
    use ocelot_semantic::compilation_inputs::CompilationInputs;
    use ocelot_semantic::symbol_table::SymbolTable;

    fn test_program_environment() -> SymbolTable {
        SymbolTable::new()
    }

    fn interpret_script(
        script: &CompilationUnit,
        source_file: &SourceFile,
        pal: &PalMock,
    ) -> OcelotResult<()> {
        let environment = test_program_environment();
        let compilation_inputs = CompilationInputs::with_default_native_functions();
        let mut script = script.clone();
        let mut context = CompilationContext::default();
        let mut environment = environment;
        resolve(
            &mut script,
            source_file,
            &mut context,
            &mut environment,
            &compilation_inputs,
        )?;
        interpret_resolved_script(&script, source_file, &environment, pal)
    }

    fn call_expression(name: &str, arguments: Vec<Expression>, span: Span) -> Expression {
        Expression::new(
            ExpressionKind::Call(CallExpression::new(
                Expression::new(
                    ExpressionKind::Identifier(Identifier::new(
                        name,
                        Span::new(span.start(), span.start() + name.len()),
                    )),
                    Span::new(span.start(), span.start() + name.len()),
                ),
                arguments,
            )),
            span,
        )
    }

    fn not_expression(operand: Expression, span: Span) -> Expression {
        Expression::new(ExpressionKind::Not(NotExpression::new(operand)), span)
    }

    fn parameter(name: &str, type_name: &str, span: Span) -> FunctionParameter {
        FunctionParameter::new(
            Identifier::new(name, Span::new(span.start(), span.start() + name.len())),
            Identifier::new(
                type_name,
                Span::new(span.end() - type_name.len(), span.end()),
            ),
            span,
        )
    }

    #[test]
    fn interprets_println_string_literal() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(Expression::new(
                        ExpressionKind::Call(CallExpression::new(
                            Expression::new(
                                ExpressionKind::Identifier(Identifier::new(
                                    "println",
                                    Span::new(0, 7),
                                )),
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
        let script = CompilationUnit::new(
            vec![
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(Expression::new(
                            ExpressionKind::Call(CallExpression::new(
                                Expression::new(
                                    ExpressionKind::Identifier(Identifier::new(
                                        "println",
                                        Span::new(0, 7),
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
                                        ExpressionKind::Identifier(Identifier::new(
                                            "println",
                                            Span::new(24, 31),
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
    fn interprets_user_defined_zero_argument_functions() {
        let script = CompilationUnit::new(
            vec![
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("greet", Span::new(4, 9)),
                        Vec::new(),
                        None,
                        None,
                        vec![Statement::new(
                            StatementKind::Expression(ExpressionStatement::new(call_expression(
                                "println",
                                vec![Expression::new(
                                    ExpressionKind::StringLiteral(StringLiteralExpression::new(
                                        "hello",
                                    )),
                                    Span::new(22, 29),
                                )],
                                Span::new(14, 30),
                            ))),
                            Span::new(14, 31),
                        )],
                        Span::new(0, 33),
                    )),
                    Span::new(0, 33),
                ),
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call_expression(
                            "greet",
                            Vec::new(),
                            Span::new(34, 41),
                        ))),
                        Span::new(34, 42),
                    )),
                    Span::new(34, 42),
                ),
            ],
            Span::new(0, 42),
        );
        let pal = PalMock::new();
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun greet() { println(\"hello\"); } greet();",
        );

        interpret_script(&script, &source_file, &pal).unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn interprets_user_defined_functions_with_string_parameters() {
        let script = CompilationUnit::new(
            vec![
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("greet", Span::new(4, 9)),
                        vec![parameter("name", "string", Span::new(10, 22))],
                        None,
                        None,
                        vec![Statement::new(
                            StatementKind::Expression(ExpressionStatement::new(call_expression(
                                "println",
                                vec![Expression::new(
                                    ExpressionKind::Identifier(Identifier::new(
                                        "name",
                                        Span::new(33, 37),
                                    )),
                                    Span::new(33, 37),
                                )],
                                Span::new(25, 38),
                            ))),
                            Span::new(25, 39),
                        )],
                        Span::new(0, 41),
                    )),
                    Span::new(0, 41),
                ),
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call_expression(
                            "greet",
                            vec![Expression::new(
                                ExpressionKind::StringLiteral(StringLiteralExpression::new(
                                    "hello",
                                )),
                                Span::new(48, 55),
                            )],
                            Span::new(42, 56),
                        ))),
                        Span::new(42, 57),
                    )),
                    Span::new(42, 57),
                ),
            ],
            Span::new(0, 57),
        );
        let pal = PalMock::new();
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun greet(name: string) { println(name); } greet(\"hello\");",
        );

        interpret_script(&script, &source_file, &pal).unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }

    #[test]
    fn interprets_user_defined_functions_with_boolean_parameters() {
        let script = CompilationUnit::new(
            vec![
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("check", Span::new(4, 9)),
                        vec![parameter("value", "bool", Span::new(10, 21))],
                        None,
                        None,
                        vec![Statement::new(
                            StatementKind::Expression(ExpressionStatement::new(call_expression(
                                "assert",
                                vec![Expression::new(
                                    ExpressionKind::Identifier(Identifier::new(
                                        "value",
                                        Span::new(31, 36),
                                    )),
                                    Span::new(31, 36),
                                )],
                                Span::new(24, 37),
                            ))),
                            Span::new(24, 38),
                        )],
                        Span::new(0, 40),
                    )),
                    Span::new(0, 40),
                ),
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call_expression(
                            "check",
                            vec![Expression::new(
                                ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true)),
                                Span::new(45, 49),
                            )],
                            Span::new(39, 50),
                        ))),
                        Span::new(39, 51),
                    )),
                    Span::new(39, 51),
                ),
            ],
            Span::new(0, 51),
        );
        let pal = PalMock::new();
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun check(value: bool) { assert(value); } check(true);",
        );

        interpret_script(&script, &source_file, &pal).unwrap();
    }

    #[test]
    fn interprets_println_boolean_literal() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(Expression::new(
                        ExpressionKind::Call(CallExpression::new(
                            Expression::new(
                                ExpressionKind::Identifier(Identifier::new(
                                    "println",
                                    Span::new(0, 7),
                                )),
                                Span::new(0, 7),
                            ),
                            vec![Expression::new(
                                ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true)),
                                Span::new(8, 12),
                            )],
                        )),
                        Span::new(0, 13),
                    ))),
                    Span::new(0, 14),
                )),
                Span::new(0, 14),
            )],
            Span::new(0, 14),
        );
        let pal = PalMock::new();
        let source_file = SourceFile::new("examples/booleans.ocelot", "println(true);");

        interpret_script(&script, &source_file, &pal).unwrap();

        assert_eq!(pal.take_printed_output(), "true\n");
    }

    #[test]
    fn interprets_println_not_false() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "println",
                        vec![not_expression(
                            Expression::new(
                                ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(
                                    false,
                                )),
                                Span::new(12, 17),
                            ),
                            Span::new(8, 17),
                        )],
                        Span::new(0, 18),
                    ))),
                    Span::new(0, 19),
                )),
                Span::new(0, 19),
            )],
            Span::new(0, 19),
        );
        let pal = PalMock::new();
        let source_file = SourceFile::new("examples/not.ocelot", "println(not false);");

        interpret_script(&script, &source_file, &pal).unwrap();

        assert_eq!(pal.take_printed_output(), "true\n");
    }

    #[test]
    fn interprets_nested_not_expressions() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "println",
                        vec![not_expression(
                            not_expression(
                                Expression::new(
                                    ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(
                                        false,
                                    )),
                                    Span::new(16, 21),
                                ),
                                Span::new(12, 21),
                            ),
                            Span::new(8, 21),
                        )],
                        Span::new(0, 22),
                    ))),
                    Span::new(0, 23),
                )),
                Span::new(0, 23),
            )],
            Span::new(0, 23),
        );
        let pal = PalMock::new();
        let source_file = SourceFile::new("examples/not.ocelot", "println(not not false);");

        interpret_script(&script, &source_file, &pal).unwrap();

        assert_eq!(pal.take_printed_output(), "false\n");
    }

    #[test]
    fn interprets_assert_eq_when_values_match() {
        let script = CompilationUnit::new(
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
    fn interprets_assert_eq_for_boolean_values() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "assert_eq",
                        vec![
                            Expression::new(
                                ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true)),
                                Span::new(10, 14),
                            ),
                            Expression::new(
                                ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true)),
                                Span::new(16, 20),
                            ),
                        ],
                        Span::new(0, 21),
                    ))),
                    Span::new(0, 22),
                )),
                Span::new(0, 22),
            )],
            Span::new(0, 22),
        );
        let source_file = SourceFile::new("examples/assertions.ocelot", "assert_eq(true, true);");
        let pal = PalMock::new();

        interpret_script(&script, &source_file, &pal).unwrap();
        assert_eq!(pal.take_printed_output(), "");
    }

    #[test]
    fn interprets_assert_when_condition_is_true() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "assert",
                        vec![Expression::new(
                            ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true)),
                            Span::new(7, 11),
                        )],
                        Span::new(0, 12),
                    ))),
                    Span::new(0, 13),
                )),
                Span::new(0, 13),
            )],
            Span::new(0, 13),
        );
        let source_file = SourceFile::new("examples/assertions.ocelot", "assert(true);");
        let pal = PalMock::new();

        interpret_script(&script, &source_file, &pal).unwrap();
        assert_eq!(pal.take_printed_output(), "");
    }

    #[test]
    fn reports_assert_false_as_an_assertion_error_without_a_diff_block() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "assert",
                        vec![Expression::new(
                            ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(false)),
                            Span::new(7, 12),
                        )],
                        Span::new(0, 13),
                    ))),
                    Span::new(0, 14),
                )),
                Span::new(0, 14),
            )],
            Span::new(0, 14),
        );
        let source_file = SourceFile::new("examples/assertions.ocelot", "assert(false);");
        let pal = PalMock::new();

        let error = interpret_script(&script, &source_file, &pal).unwrap_err();

        let ErrorKind::AssertionError(assertion_error) = error.kind() else {
            panic!("expected assertion error, got {:?}", error.kind());
        };
        assert_eq!(assertion_error.summary(), "assert condition was false");
        assert_eq!(assertion_error.expected, None);
        assert_eq!(assertion_error.actual, None);
        assert!(!error.to_test_string().contains("expected:"));
        assert!(!error.to_test_string().contains("actual:"));
    }

    #[test]
    fn interprets_assert_with_a_not_expression() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "assert",
                        vec![not_expression(
                            Expression::new(
                                ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(
                                    false,
                                )),
                                Span::new(11, 16),
                            ),
                            Span::new(7, 16),
                        )],
                        Span::new(0, 17),
                    ))),
                    Span::new(0, 18),
                )),
                Span::new(0, 18),
            )],
            Span::new(0, 18),
        );
        let source_file = SourceFile::new("examples/not.ocelot", "assert(not false);");
        let pal = PalMock::new();

        interpret_script(&script, &source_file, &pal).unwrap();
        assert_eq!(pal.take_printed_output(), "");
    }

    #[test]
    fn reports_assert_not_true_as_a_minimal_assertion_error() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "assert",
                        vec![not_expression(
                            Expression::new(
                                ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true)),
                                Span::new(11, 15),
                            ),
                            Span::new(7, 15),
                        )],
                        Span::new(0, 16),
                    ))),
                    Span::new(0, 17),
                )),
                Span::new(0, 17),
            )],
            Span::new(0, 17),
        );
        let source_file = SourceFile::new("examples/not.ocelot", "assert(not true);");
        let pal = PalMock::new();

        let error = interpret_script(&script, &source_file, &pal).unwrap_err();

        let ErrorKind::AssertionError(assertion_error) = error.kind() else {
            panic!("expected assertion error, got {:?}", error.kind());
        };
        assert_eq!(assertion_error.summary(), "assert condition was false");
        assert_eq!(assertion_error.expected, None);
        assert_eq!(assertion_error.actual, None);
    }

    #[test]
    fn reports_assert_eq_mismatches_as_assertion_errors() {
        let script = CompilationUnit::new(
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
    fn reports_boolean_assert_eq_mismatches_with_boolean_values() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "assert_eq",
                        vec![
                            Expression::new(
                                ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true)),
                                Span::new(10, 14),
                            ),
                            Expression::new(
                                ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(
                                    false,
                                )),
                                Span::new(16, 21),
                            ),
                        ],
                        Span::new(0, 22),
                    ))),
                    Span::new(0, 23),
                )),
                Span::new(0, 23),
            )],
            Span::new(0, 23),
        );
        let source_file = SourceFile::new("examples/assertions.ocelot", "assert_eq(true, false);");
        let pal = PalMock::new();

        let error = interpret_script(&script, &source_file, &pal).unwrap_err();

        let ErrorKind::AssertionError(assertion_error) = error.kind() else {
            panic!("expected assertion error, got {:?}", error.kind());
        };
        assert_eq!(
            assertion_error.expected.as_ref().map(|s| s.as_str()),
            Some("true")
        );
        assert_eq!(
            assertion_error.actual.as_ref().map(|s| s.as_str()),
            Some("false")
        );
    }

    #[test]
    fn reports_assert_eq_wrong_arity() {
        let script = CompilationUnit::new(
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

    #[test]
    fn reports_assert_wrong_arity() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "assert",
                        vec![],
                        Span::new(0, 8),
                    ))),
                    Span::new(0, 9),
                )),
                Span::new(0, 9),
            )],
            Span::new(0, 9),
        );
        let source_file = SourceFile::new("examples/assertions.ocelot", "assert();");
        let pal = PalMock::new();

        let error = interpret_script(&script, &source_file, &pal).unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("type error: `assert` expects exactly one argument")
        );
    }

    #[test]
    fn reports_assert_wrong_type() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "assert",
                        vec![Expression::new(
                            ExpressionKind::StringLiteral(StringLiteralExpression::new("hello")),
                            Span::new(7, 14),
                        )],
                        Span::new(0, 15),
                    ))),
                    Span::new(0, 16),
                )),
                Span::new(0, 16),
            )],
            Span::new(0, 16),
        );
        let source_file = SourceFile::new("examples/assertions.ocelot", "assert(\"hello\");");
        let pal = PalMock::new();

        let error = interpret_script(&script, &source_file, &pal).unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(ocelot_base::compilation_stage::CompilationStage::Resolver)
        ));
        assert!(
            error
                .to_test_string()
                .contains("type error: `assert` expects a bool argument")
        );
    }

    #[test]
    fn reports_not_wrong_type() {
        let script = CompilationUnit::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call_expression(
                        "println",
                        vec![not_expression(
                            Expression::new(
                                ExpressionKind::StringLiteral(StringLiteralExpression::new(
                                    "hello",
                                )),
                                Span::new(12, 19),
                            ),
                            Span::new(8, 19),
                        )],
                        Span::new(0, 20),
                    ))),
                    Span::new(0, 21),
                )),
                Span::new(0, 21),
            )],
            Span::new(0, 21),
        );
        let source_file = SourceFile::new("examples/not.ocelot", "println(not \"hello\");");
        let pal = PalMock::new();

        let error = interpret_script(&script, &source_file, &pal).unwrap_err();

        assert!(matches!(
            error.kind(),
            ErrorKind::CompilationError(ocelot_base::compilation_stage::CompilationStage::Resolver)
        ));
        assert!(
            error
                .to_test_string()
                .contains("operator `not` expects a bool operand")
        );
    }
}
