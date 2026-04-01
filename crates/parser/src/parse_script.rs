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
    use ocelot_ast::statement_kind::StatementKind;
    use ocelot_base::source_file::SourceFile;

    #[test]
    fn parses_println_string_statement() {
        let source_file = SourceFile::new("examples/hello.ocelot", "println(\"hello\");");

        let script = parse_script(&source_file).unwrap();

        assert_eq!(script.statements.len(), 1);

        let statement = &script.statements[0];
        match &statement.kind {
            StatementKind::Println(println_statement) => match &println_statement.argument.kind {
                ExpressionKind::StringLiteral(string_literal) => {
                    assert_eq!(string_literal.value, "hello");
                }
                other => panic!("expected string literal, got {other:?}"),
            },
        }
    }

    #[test]
    fn parses_multiple_println_statements() {
        let source_file = SourceFile::new(
            "examples/two-lines.ocelot",
            "println(\"first\"); println(\"second\");",
        );

        let script = parse_script(&source_file).unwrap();

        assert_eq!(script.statements.len(), 2);
        assert_eq!(script.span.start(), 0);
        assert_eq!(script.span.end(), source_file.source().len());
    }
}
