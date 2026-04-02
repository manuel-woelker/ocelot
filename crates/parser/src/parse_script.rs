use crate::parser::Parser;
use ocelot_ast::script::Script;
use ocelot_base::result::OcelotResult;
use ocelot_base::source_file::SourceFile;

/// Parses a source file into a script AST.
pub fn parse_script(source_file: &SourceFile) -> OcelotResult<Script> {
    Parser::new(source_file)?.parse_script()
}

#[cfg(test)]
mod tests {
    use super::parse_script;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::item_kind::ItemKind;
    use ocelot_ast::statement_kind::StatementKind;
    use ocelot_base::source_file::SourceFile;

    #[test]
    fn parses_println_string_statement() {
        let source_file = SourceFile::new("examples/hello.ocelot", "println(\"hello\");");

        let script = parse_script(&source_file).unwrap();

        assert_eq!(script.items.len(), 1);

        match &script.items[0].kind {
            ItemKind::Statement(statement) => match &statement.kind {
                StatementKind::Println(println_statement) => match &println_statement.argument.kind
                {
                    ExpressionKind::StringLiteral(string_literal) => {
                        assert_eq!(string_literal.value, "hello");
                    }
                    other => panic!("expected string literal, got {other:?}"),
                },
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

        let script = parse_script(&source_file).unwrap();

        assert_eq!(script.items.len(), 2);
        assert_eq!(script.span.start(), 0);
        assert_eq!(script.span.end(), source_file.source().len());
    }

    #[test]
    fn parses_test_items_alongside_script_statements() {
        let source_file = SourceFile::new(
            "examples/tests.ocelot",
            "println(\"setup\"); test \"prints one line\" { println(\"hello\"); }",
        );

        let script = parse_script(&source_file).unwrap();

        assert_eq!(script.items.len(), 2);

        match &script.items[1].kind {
            ItemKind::Test(test_item) => {
                assert_eq!(test_item.name, "prints one line");
                assert_eq!(test_item.body.len(), 1);
            }
            other => panic!("expected test item, got {other:?}"),
        }
    }

    #[test]
    fn reports_a_clear_error_for_missing_test_name() {
        let source_file = SourceFile::new("examples/invalid.ocelot", "test { println(\"x\"); }");

        let error = parse_script(&source_file).unwrap_err();

        assert_eq!(error.kind().to_string(), "expected test name string");
    }

    #[test]
    fn reports_a_clear_error_for_unterminated_test_body() {
        let source_file = SourceFile::new(
            "examples/invalid.ocelot",
            "test \"broken\" { println(\"hello\");",
        );

        let error = parse_script(&source_file).unwrap_err();

        assert_eq!(error.kind().to_string(), "expected `}` to close test body");
    }

    #[test]
    fn reports_a_clear_error_for_zero_argument_println() {
        let source_file = SourceFile::new("examples/invalid.ocelot", "println();");

        let error = parse_script(&source_file).unwrap_err();

        assert_eq!(
            error.kind().to_string(),
            "type error: `println` expects exactly one argument"
        );
    }
}
