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
    use ocelot_ast::call_expression::CallExpression;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::expression_statement::ExpressionStatement;
    use ocelot_ast::item_kind::ItemKind;
    use ocelot_ast::statement_kind::StatementKind;
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
                        ExpressionKind::Call(CallExpression { callee, arguments }) => {
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
}
