use crate::parser::Parser;
use ocelot_ast::script::Script;
use ocelot_base::compilation_context::CompilationContext;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::error::OcelotError;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::OcelotResult;
use ocelot_base::source_file::SourceFile;

/// Parses a source file into a script AST.
pub fn parse_script(
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
) -> OcelotResult<Script> {
    match Parser::new(source_file, compilation_context).parse_script() {
        Ok(script) => Ok(script),
        Err(_) if compilation_context.has_errors() => Err(OcelotError::compilation_error(
            CompilationStage::Parser,
        )
        .with_source(OcelotError::message(render_source_diagnostics(
            &compilation_context.source_diagnostics.diagnostics,
        )))),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_script;
    use ocelot_ast::boolean_literal_expression::BooleanLiteralExpression;
    use ocelot_ast::call_expression::CallExpression;
    use ocelot_ast::expression::Expression;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::expression_statement::ExpressionStatement;
    use ocelot_ast::item::Item;
    use ocelot_ast::item_kind::ItemKind;
    use ocelot_ast::not_expression::NotExpression;
    use ocelot_ast::statement_kind::StatementKind;
    use ocelot_ast::type_index::TypeIndex;
    use ocelot_base::compilation_context::CompilationContext;
    use ocelot_base::diagnostic_level::DiagnosticLevel;
    use ocelot_base::source_file::SourceFile;
    use ocelot_base::span::Span;

    #[test]
    fn parses_println_call_expression_statement() {
        let source_file = SourceFile::new("examples/hello.ocelot", "println(\"hello\");");
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(!context.has_errors());

        match &script.items[0].kind {
            ItemKind::Statement(statement) => match &statement.kind {
                StatementKind::Expression(ExpressionStatement { expression }) => {
                    match &expression.kind {
                        ExpressionKind::Call(CallExpression {
                            callee, arguments, ..
                        }) => {
                            match &callee.kind {
                                ExpressionKind::Identifier(identifier) => {
                                    assert_eq!(identifier.name, "println");
                                }
                                other => panic!("expected identifier callee, got {other:?}"),
                            }

                            assert_eq!(arguments.len(), 1);

                            match &arguments[0].kind {
                                ExpressionKind::StringLiteral(string_literal) => {
                                    assert_eq!(string_literal.value, "hello");
                                }
                                other => panic!("expected string literal, got {other:?}"),
                            }
                        }
                        other => panic!("expected call expression, got {other:?}"),
                    }
                }
            },
            other => panic!("expected statement item, got {other:?}"),
        }
    }

    #[test]
    fn parses_multiple_println_statements() {
        let source_file = SourceFile::new(
            "examples/two-lines.ocelot",
            "println(\"first\"); println(\"second\");",
        );
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 2);
        assert_eq!(script.span.start(), 0);
        assert_eq!(script.span.end(), source_file.source().len());
        assert!(!context.has_errors());
    }

    #[test]
    fn parses_true_as_a_boolean_literal_expression() {
        let source_file = SourceFile::new("examples/booleans.ocelot", "true;");
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(!context.has_errors());

        match &script.items[0].kind {
            ItemKind::Statement(statement) => match &statement.kind {
                StatementKind::Expression(ExpressionStatement { expression }) => {
                    assert_eq!(
                        expression.kind,
                        ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true))
                    );
                }
            },
            other => panic!("expected statement item, got {other:?}"),
        }
    }

    #[test]
    fn parses_false_as_a_boolean_literal_expression() {
        let source_file = SourceFile::new("examples/booleans.ocelot", "false;");
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(!context.has_errors());

        match &script.items[0].kind {
            ItemKind::Statement(statement) => match &statement.kind {
                StatementKind::Expression(ExpressionStatement { expression }) => {
                    assert_eq!(
                        expression.kind,
                        ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(false))
                    );
                }
            },
            other => panic!("expected statement item, got {other:?}"),
        }
    }

    #[test]
    fn parsed_expressions_start_with_unresolved_type_metadata() {
        let source_file = SourceFile::new("examples/types.ocelot", "not false;");
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        let ItemKind::Statement(statement) = &script.items[0].kind else {
            panic!("expected statement item");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
        let ExpressionKind::Not(NotExpression { operand }) = &expression.kind else {
            panic!("expected not expression");
        };

        assert_eq!(expression.ty, TypeIndex::unresolved());
        assert_eq!(operand.ty, TypeIndex::unresolved());
    }

    #[test]
    fn parses_boolean_literals_as_call_arguments() {
        let source_file = SourceFile::new("examples/booleans.ocelot", "assert_eq(true, false);");
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(!context.has_errors());

        match &script.items[0].kind {
            ItemKind::Statement(statement) => match &statement.kind {
                StatementKind::Expression(ExpressionStatement { expression }) => {
                    let ExpressionKind::Call(CallExpression { arguments, .. }) = &expression.kind
                    else {
                        panic!("expected call expression, got {:?}", expression.kind);
                    };

                    assert_eq!(
                        arguments[0].kind,
                        ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true))
                    );
                    assert_eq!(
                        arguments[1].kind,
                        ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(false))
                    );
                }
            },
            other => panic!("expected statement item, got {other:?}"),
        }
    }

    #[test]
    fn parses_not_true_as_a_not_expression() {
        let source_file = SourceFile::new("examples/not.ocelot", "not true;");
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(!context.has_errors());

        match &script.items[0].kind {
            ItemKind::Statement(statement) => match &statement.kind {
                StatementKind::Expression(ExpressionStatement { expression }) => {
                    assert_eq!(
                        expression.kind,
                        ExpressionKind::Not(NotExpression::new(Expression::new(
                            ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true)),
                            Span::new(4, 8),
                        )))
                    );
                }
            },
            other => panic!("expected statement item, got {other:?}"),
        }
    }

    #[test]
    fn parses_nested_not_expressions() {
        let source_file = SourceFile::new("examples/not.ocelot", "not not false;");
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(!context.has_errors());

        match &script.items[0].kind {
            ItemKind::Statement(statement) => match &statement.kind {
                StatementKind::Expression(ExpressionStatement { expression }) => {
                    assert_eq!(
                        expression.kind,
                        ExpressionKind::Not(NotExpression::new(Expression::new(
                            ExpressionKind::Not(NotExpression::new(Expression::new(
                                ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(
                                    false,
                                )),
                                Span::new(8, 13),
                            ))),
                            Span::new(4, 13),
                        )))
                    );
                }
            },
            other => panic!("expected statement item, got {other:?}"),
        }
    }

    #[test]
    fn parses_not_with_calls_binding_tighter_than_the_operator() {
        let source_file = SourceFile::new("examples/not.ocelot", "not foo();");
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(!context.has_errors());

        match &script.items[0].kind {
            ItemKind::Statement(statement) => match &statement.kind {
                StatementKind::Expression(ExpressionStatement { expression }) => {
                    let ExpressionKind::Not(NotExpression { operand }) = &expression.kind else {
                        panic!("expected not expression, got {:?}", expression.kind);
                    };

                    let ExpressionKind::Call(CallExpression {
                        callee, arguments, ..
                    }) = &operand.kind
                    else {
                        panic!("expected call operand, got {:?}", operand.kind);
                    };

                    assert!(arguments.is_empty());

                    let ExpressionKind::Identifier(identifier) = &callee.kind else {
                        panic!("expected identifier callee, got {:?}", callee.kind);
                    };
                    assert_eq!(identifier.name, "foo");
                }
            },
            other => panic!("expected statement item, got {other:?}"),
        }
    }

    #[test]
    fn parses_not_inside_call_arguments() {
        let source_file = SourceFile::new("examples/not.ocelot", "assert(not false);");
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(!context.has_errors());

        match &script.items[0].kind {
            ItemKind::Statement(statement) => match &statement.kind {
                StatementKind::Expression(ExpressionStatement { expression }) => {
                    let ExpressionKind::Call(CallExpression { arguments, .. }) = &expression.kind
                    else {
                        panic!("expected call expression, got {:?}", expression.kind);
                    };

                    assert_eq!(
                        arguments[0].kind,
                        ExpressionKind::Not(NotExpression::new(Expression::new(
                            ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(false)),
                            Span::new(11, 16),
                        )))
                    );
                }
            },
            other => panic!("expected statement item, got {other:?}"),
        }
    }

    #[test]
    fn parses_scripts_with_comments_without_changing_the_item_shape() {
        let source_file = SourceFile::new(
            "examples/comments.ocelot",
            "// setup\nprintln(/* callee gap */\"first\"); /* between */ println(\"second\");",
        );
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 2);
        assert!(!context.has_errors());
    }

    #[test]
    fn parses_function_items_alongside_script_statements() {
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun greet() { println(\"hello\"); } greet();",
        );
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 2);
        assert!(!context.has_errors());

        match &script.items[0].kind {
            ItemKind::Function(function_item) => {
                assert_eq!(function_item.identifier.name, "greet");
                assert_eq!(function_item.body.len(), 1);
            }
            other => panic!("expected function item, got {other:?}"),
        }
    }

    #[test]
    fn parses_multiple_function_items() {
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun first() {} fun second() {}",
        );
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 2);
        assert!(matches!(
            script.items[0],
            Item {
                kind: ItemKind::Function(_),
                ..
            }
        ));
        assert!(matches!(
            script.items[1],
            Item {
                kind: ItemKind::Function(_),
                ..
            }
        ));
    }

    #[test]
    fn parses_test_items_alongside_script_statements() {
        let source_file = SourceFile::new(
            "examples/tests.ocelot",
            "println(\"setup\"); test \"prints one line\" { println(\"hello\"); }",
        );
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 2);
        assert!(!context.has_errors());

        match &script.items[1].kind {
            ItemKind::Function(_) => panic!("expected test item, got function item"),
            ItemKind::Test(test_item) => {
                assert_eq!(test_item.name, "prints one line");
                assert_eq!(test_item.body.len(), 1);
            }
            other => panic!("expected test item, got {other:?}"),
        }
    }

    #[test]
    fn parses_test_items_with_comments_around_the_name_and_body() {
        let source_file = SourceFile::new(
            "examples/tests.ocelot",
            "test /* name */ \"prints one line\" /* body */ { // enter body\n println(\"hello\"); /* done */ }",
        );
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(!context.has_errors());

        match &script.items[0].kind {
            ItemKind::Function(_) => panic!("expected test item, got function item"),
            ItemKind::Test(test_item) => {
                assert_eq!(test_item.name, "prints one line");
                assert_eq!(test_item.body.len(), 1);
            }
            other => panic!("expected test item, got {other:?}"),
        }
    }

    #[test]
    fn reports_a_missing_test_name_as_a_source_diagnostic() {
        let source_file = SourceFile::new("examples/invalid.ocelot", "test { println(\"x\"); }");
        let mut context = CompilationContext::default();

        parse_script(&source_file, &mut context).unwrap_err();
        assert!(context.has_errors());
        assert_eq!(context.source_diagnostics.diagnostics.len(), 1);
        assert_eq!(
            context.source_diagnostics.diagnostics[0].message,
            "expected test name string"
        );
        assert_eq!(
            context.source_diagnostics.diagnostics[0].excerpts[0].annotations[0].span,
            Span::new(5, 6)
        );
    }

    #[test]
    fn reports_a_missing_function_name_as_a_source_diagnostic() {
        let source_file = SourceFile::new("examples/invalid.ocelot", "fun () { println(\"x\"); }");
        let mut context = CompilationContext::default();

        parse_script(&source_file, &mut context).unwrap_err();
        assert!(context.has_errors());
        assert_eq!(context.source_diagnostics.diagnostics.len(), 1);
        assert_eq!(
            context.source_diagnostics.diagnostics[0].message,
            "expected function name"
        );
    }

    #[test]
    fn reports_an_unterminated_function_body_as_a_source_diagnostic() {
        let source_file = SourceFile::new(
            "examples/invalid.ocelot",
            "fun greet() { println(\"hello\");",
        );
        let mut context = CompilationContext::default();

        parse_script(&source_file, &mut context).unwrap_err();
        assert!(context.has_errors());
        assert_eq!(context.source_diagnostics.diagnostics.len(), 1);
        assert_eq!(
            context.source_diagnostics.diagnostics[0].message,
            "expected `}` to close function body"
        );
    }

    #[test]
    fn reports_an_unterminated_test_body_as_a_source_diagnostic() {
        let source_file = SourceFile::new(
            "examples/invalid.ocelot",
            "test \"broken\" { println(\"hello\");",
        );
        let mut context = CompilationContext::default();

        parse_script(&source_file, &mut context).unwrap_err();
        assert!(context.has_errors());
        assert_eq!(context.source_diagnostics.diagnostics.len(), 1);
        assert_eq!(
            context.source_diagnostics.diagnostics[0].message,
            "expected `}` to close test body"
        );
    }

    #[test]
    fn parses_non_call_expression_statements() {
        let source_file = SourceFile::new("examples/statement.ocelot", "\"hello\"; name;");
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 2);
        assert!(!context.has_errors());
    }

    #[test]
    fn parses_multiple_call_arguments() {
        let source_file = SourceFile::new(
            "examples/arguments.ocelot",
            "println(\"hello\", \"world\");",
        );
        let mut context = CompilationContext::default();

        parse_script(&source_file, &mut context).unwrap_err();
        assert!(context.has_errors());
        assert_eq!(
            context.source_diagnostics.diagnostics[0].message,
            "type error: `println` expects exactly one argument"
        );
        assert_eq!(
            context.source_diagnostics.diagnostics[0].excerpts[0].annotations[0].message,
            "extra argument"
        );
    }

    #[test]
    fn reports_zero_argument_println_as_a_source_diagnostic() {
        let source_file = SourceFile::new("examples/invalid.ocelot", "println();");
        let mut context = CompilationContext::default();

        parse_script(&source_file, &mut context).unwrap_err();
        assert!(context.has_errors());
        assert_eq!(
            context.source_diagnostics.diagnostics[0].message,
            "type error: `println` expects exactly one argument"
        );
    }

    #[test]
    fn surfaces_lexer_diagnostics_through_the_shared_compilation_context() {
        let source_file = SourceFile::new("examples/invalid.ocelot", "println(\"hello);");
        let mut context = CompilationContext::default();

        parse_script(&source_file, &mut context).unwrap_err();
        assert!(context.has_errors());
        assert_eq!(context.source_diagnostics.diagnostics.len(), 1);
        assert_eq!(
            context.source_diagnostics.diagnostics[0].level,
            DiagnosticLevel::Error
        );
        assert_eq!(
            context.source_diagnostics.diagnostics[0].message,
            "unterminated string literal"
        );
    }

    #[test]
    fn surfaces_unterminated_block_comment_diagnostics_through_the_shared_compilation_context() {
        let source_file = SourceFile::new("examples/invalid.ocelot", "println(/* hello");
        let mut context = CompilationContext::default();

        parse_script(&source_file, &mut context).unwrap_err();
        assert!(context.has_errors());
        assert_eq!(context.source_diagnostics.diagnostics.len(), 1);
        assert_eq!(
            context.source_diagnostics.diagnostics[0].level,
            DiagnosticLevel::Error
        );
        assert_eq!(
            context.source_diagnostics.diagnostics[0].message,
            "unterminated block comment"
        );
    }

    #[test]
    fn reports_trailing_commas_in_argument_lists() {
        let source_file = SourceFile::new("examples/invalid.ocelot", "println(\"hello\",);");
        let mut context = CompilationContext::default();

        parse_script(&source_file, &mut context).unwrap_err();
        assert!(context.has_errors());
        assert_eq!(
            context.source_diagnostics.diagnostics[0].message,
            "expected expression"
        );
    }

    #[test]
    fn parses_effect_items() {
        let source_file = SourceFile::new("examples/effects.ocelot", "effect exec;");
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(!context.has_errors());
        match &script.items[0].kind {
            ItemKind::Effect(effect_item) => {
                assert_eq!(effect_item.identifier.name, "exec");
            }
            other => panic!("expected effect item, got {other:?}"),
        }
    }

    #[test]
    fn parses_function_items_with_can_and_cannot_effect_clauses() {
        let source_file = SourceFile::new(
            "examples/effects.ocelot",
            "fun greet() can exec cannot panic {}",
        );
        let mut context = CompilationContext::default();

        let script = parse_script(&source_file, &mut context).unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(!context.has_errors());
        match &script.items[0].kind {
            ItemKind::Function(function_item) => {
                assert_eq!(
                    function_item
                        .can_clause
                        .as_ref()
                        .expect("can clause should be present")
                        .effect
                        .name,
                    "exec"
                );
                assert_eq!(
                    function_item
                        .cannot_clause
                        .as_ref()
                        .expect("cannot clause should be present")
                        .effect
                        .name,
                    "panic"
                );
            }
            other => panic!("expected function item, got {other:?}"),
        }
    }

    #[test]
    fn rejects_function_effect_clauses_in_the_wrong_order() {
        let source_file = SourceFile::new(
            "examples/invalid.ocelot",
            "fun greet() cannot panic can exec {}",
        );
        let mut context = CompilationContext::default();

        parse_script(&source_file, &mut context).unwrap_err();

        assert!(context.has_errors());
        assert_eq!(
            context.source_diagnostics.diagnostics[0].message,
            "function effect clauses must place `can` before `cannot`"
        );
    }
}
