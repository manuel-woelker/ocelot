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
    use ocelot_ast::expression::Expression;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::println_statement::PrintlnStatement;
    use ocelot_ast::script::Script;
    use ocelot_ast::statement::Statement;
    use ocelot_ast::statement_kind::StatementKind;
    use ocelot_ast::string_literal_expression::StringLiteralExpression;
    use ocelot_base::span::Span;
    use ocelot_pal::pal_mock::PalMock;

    #[test]
    fn interprets_println_string_literal() {
        let script = Script::new(
            vec![Statement::new(
                StatementKind::Println(PrintlnStatement::new(Expression::new(
                    ExpressionKind::StringLiteral(StringLiteralExpression::new("hello")),
                    Span::new(8, 15),
                ))),
                Span::new(0, 17),
            )],
            Span::new(0, 17),
        );
        let pal = PalMock::new();

        interpret_script(&script, &pal).unwrap();

        assert_eq!(pal.take_printed_output(), "hello\n");
    }
}
