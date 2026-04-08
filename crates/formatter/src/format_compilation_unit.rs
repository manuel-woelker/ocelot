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
use ocelot_ast::trivia_piece::TriviaPiece;
use ocelot_ast::use_item::UseItem;

const INDENT: &str = "    ";

/// Formats one compilation unit into stable source text.
pub fn format_compilation_unit(compilation_unit: &CompilationUnit) -> String {
    let mut output = String::new();
    write_compilation_unit_leading_trivia(&mut output, compilation_unit);

    for (index, item) in compilation_unit.items.iter().enumerate() {
        if index > 0 {
            output.push('\n');
            write_blank_lines(&mut output, blank_line_count(&item.trivia.leading));
        }

        write_leading_comments(&mut output, &item.trivia.leading, 0);
        write_item(&mut output, item, 0);
        write_trailing_comments(&mut output, &item.trivia.trailing);
    }

    write_compilation_unit_trailing_trivia(&mut output, compilation_unit);
    if output.ends_with('\n') {
        output.pop();
    }
    output
}

fn write_compilation_unit_leading_trivia(output: &mut String, compilation_unit: &CompilationUnit) {
    write_blank_lines(output, blank_line_count(&compilation_unit.trivia.leading));
    write_leading_comments(output, &compilation_unit.trivia.leading, 0);
}

fn write_compilation_unit_trailing_trivia(output: &mut String, compilation_unit: &CompilationUnit) {
    if compilation_unit.trivia.trailing.is_empty() {
        return;
    }

    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    write_blank_lines(output, blank_line_count(&compilation_unit.trivia.trailing));
    write_leading_comments(output, &compilation_unit.trivia.trailing, 0);
}

fn write_item(output: &mut String, item: &Item, indent_level: usize) {
    match &item.kind {
        ItemKind::Effect(effect_item) => write_effect_item(output, effect_item, indent_level),
        ItemKind::Function(function_item) => {
            write_function_item(output, function_item, indent_level);
        }
        ItemKind::Statement(statement) => write_statement(output, statement, indent_level),
        ItemKind::Test(test_item) => write_test_item(output, test_item, indent_level),
        ItemKind::Use(use_item) => write_use_item(output, use_item, indent_level),
    }
}

fn write_effect_item(output: &mut String, effect_item: &EffectItem, indent_level: usize) {
    write_indent(output, indent_level);
    output.push_str("effect ");
    output.push_str(effect_item.identifier.name.as_str());
    output.push(';');
}

fn write_function_item(output: &mut String, function_item: &FunctionItem, indent_level: usize) {
    write_indent(output, indent_level);

    if function_item.is_native {
        output.push_str("native ");
    }

    output.push_str("fun ");
    output.push_str(function_item.identifier.name.as_str());
    output.push('(');

    for (index, parameter) in function_item.parameters.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        write_function_parameter(output, parameter);
    }

    output.push(')');

    if let Some(can_clause) = &function_item.can_clause {
        output.push(' ');
        write_function_effect_clause(output, "can", can_clause);
    }

    if let Some(cannot_clause) = &function_item.cannot_clause {
        output.push(' ');
        write_function_effect_clause(output, "cannot", cannot_clause);
    }

    if function_item.is_native {
        output.push(';');
        return;
    }

    if function_item.body.is_empty() {
        output.push_str(" {}");
        return;
    }

    output.push_str(" {\n");
    write_statement_list(output, &function_item.body, indent_level + 1);
    output.push('\n');
    write_indent(output, indent_level);
    output.push('}');
}

fn write_function_parameter(output: &mut String, parameter: &FunctionParameter) {
    output.push_str(parameter.identifier.name.as_str());
    output.push_str(": ");
    output.push_str(parameter.type_name.name.as_str());
}

fn write_function_effect_clause(output: &mut String, keyword: &str, clause: &FunctionEffectClause) {
    output.push_str(keyword);
    output.push(' ');
    output.push_str(clause.effect.name.as_str());
}

fn write_test_item(output: &mut String, test_item: &TestItem, indent_level: usize) {
    write_indent(output, indent_level);
    output.push_str("test ");
    output.push('"');
    output.push_str(test_item.name.as_str());
    output.push('"');

    if test_item.body.is_empty() {
        output.push_str(" {}");
        return;
    }

    output.push_str(" {\n");
    write_statement_list(output, &test_item.body, indent_level + 1);
    output.push('\n');
    write_indent(output, indent_level);
    output.push('}');
}

fn write_use_item(output: &mut String, use_item: &UseItem, indent_level: usize) {
    write_indent(output, indent_level);
    output.push_str("use ");
    output.push_str(use_item.module_path.render().as_str());
    output.push_str("::");

    if use_item.imported_names.len() == 1 {
        output.push_str(use_item.imported_names[0].name.as_str());
    } else {
        output.push('{');
        for (index, identifier) in use_item.imported_names.iter().enumerate() {
            if index > 0 {
                output.push_str(", ");
            }
            output.push_str(identifier.name.as_str());
        }
        output.push('}');
    }

    output.push(';');
}

fn write_statement_list(output: &mut String, statements: &[Statement], indent_level: usize) {
    for (index, statement) in statements.iter().enumerate() {
        if index > 0 {
            output.push('\n');
            write_blank_lines(output, blank_line_count(&statement.trivia.leading));
        }

        write_leading_comments(output, &statement.trivia.leading, indent_level);
        write_statement(output, statement, indent_level);
        write_trailing_comments(output, &statement.trivia.trailing);
    }
}

fn write_statement(output: &mut String, statement: &Statement, indent_level: usize) {
    match &statement.kind {
        StatementKind::Expression(ExpressionStatement { expression }) => {
            write_indent(output, indent_level);
            write_expression(output, expression);
            output.push(';');
        }
    }
}

fn write_expression(output: &mut String, expression: &Expression) {
    match &expression.kind {
        ExpressionKind::BooleanLiteral(BooleanLiteralExpression { value }) => {
            output.push_str(if *value { "true" } else { "false" });
        }
        ExpressionKind::Call(CallExpression {
            callee, arguments, ..
        }) => {
            write_expression(output, callee);
            output.push('(');
            for (index, argument) in arguments.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                write_expression(output, argument);
            }
            output.push(')');
        }
        ExpressionKind::Identifier(Identifier { name, .. }) => {
            output.push_str(name.as_str());
        }
        ExpressionKind::Not(NotExpression { operand }) => {
            output.push_str("not ");
            write_expression(output, operand);
        }
        ExpressionKind::QualifiedIdentifier(QualifiedIdentifier { segments }) => {
            for (index, segment) in segments.iter().enumerate() {
                if index > 0 {
                    output.push_str("::");
                }
                output.push_str(segment.name.as_str());
            }
        }
        ExpressionKind::StringLiteral(StringLiteralExpression { value }) => {
            output.push('"');
            output.push_str(value);
            output.push('"');
        }
        ExpressionKind::TemplateString(TemplateStringExpression { parts }) => {
            output.push('"');
            for part in parts {
                match part {
                    TemplateStringPart::Interpolation(expression) => {
                        output.push_str("${");
                        write_expression(output, expression);
                        output.push('}');
                    }
                    TemplateStringPart::Text(text) => output.push_str(text.as_str()),
                }
            }
            output.push('"');
        }
    }
}

fn write_leading_comments(output: &mut String, pieces: &[TriviaPiece], indent_level: usize) {
    for piece in pieces {
        match piece {
            TriviaPiece::Newlines { .. } => {}
            TriviaPiece::LineComment { text, .. } | TriviaPiece::BlockComment { text, .. } => {
                write_indented_text(output, text.as_str(), indent_level);
                output.push('\n');
            }
        }
    }
}

fn write_trailing_comments(output: &mut String, pieces: &[TriviaPiece]) {
    for piece in pieces {
        match piece {
            TriviaPiece::LineComment { text, .. } | TriviaPiece::BlockComment { text, .. } => {
                output.push(' ');
                output.push_str(text.as_str());
            }
            TriviaPiece::Newlines { .. } => {}
        }
    }
}

fn write_indented_text(output: &mut String, text: &str, indent_level: usize) {
    for (index, line) in text.lines().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        write_indent(output, indent_level);
        output.push_str(line);
    }
}

fn blank_line_count(pieces: &[TriviaPiece]) -> usize {
    pieces
        .iter()
        .filter_map(|piece| match piece {
            TriviaPiece::Newlines { count, .. } => Some(count.saturating_sub(1)),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

fn write_blank_lines(output: &mut String, count: usize) {
    for _ in 0..count {
        output.push('\n');
    }
}

fn write_indent(output: &mut String, indent_level: usize) {
    for _ in 0..indent_level {
        output.push_str(INDENT);
    }
}

#[cfg(test)]
mod tests {
    use super::format_compilation_unit;
    use ocelot_base::source_diagnostics::SourceDiagnostics;
    use ocelot_base::source_file::SourceFile;
    use ocelot_parser::parse_compilation_unit::parse_compilation_unit;

    fn parse(source: &str) -> ocelot_ast::compilation_unit::CompilationUnit {
        let source_file = SourceFile::new("examples/format.ocelot", source);
        let mut source_diagnostics = SourceDiagnostics::default();
        let compilation_unit = parse_compilation_unit(&source_file, &mut source_diagnostics)
            .expect("source should parse");
        assert!(
            !source_diagnostics.has_errors(),
            "unexpected diagnostics: {:?}",
            source_diagnostics.diagnostics
        );
        compilation_unit
    }

    #[test]
    fn formats_comment_heavy_supported_inputs() {
        let input = "// setup\nuse math::trig::{cos, sin}; // imports\n\nfun greet(name: string) {\n// before call\nprintln(not helper::call(name)); // after call\n}\n\n// test setup\ntest \"works\" {\nprintln(\"ok\"); // done\n}\n";

        let formatted = format_compilation_unit(&parse(input));

        assert_eq!(
            formatted,
            "// setup\nuse math::trig::{cos, sin}; // imports\n\nfun greet(name: string) {\n    // before call\n    println(not helper::call(name)); // after call\n}\n\n// test setup\ntest \"works\" {\n    println(\"ok\"); // done\n}"
        );
    }

    #[test]
    fn formatting_is_idempotent_for_supported_comment_positions() {
        let input = "// header\neffect exec;\n\nfun greet() {\n// note\nprintln(\"hi\"); // trailing\n}\n// footer";

        let once = format_compilation_unit(&parse(input));
        let twice = format_compilation_unit(&parse(&once));

        assert_eq!(once, twice);
    }

    #[test]
    fn formats_template_strings() {
        let input = "fun greet(name: string) {\nprintln(\"Hello ${ name } ${not false}\");\n}";

        let formatted = format_compilation_unit(&parse(input));

        assert_eq!(
            formatted,
            "fun greet(name: string) {\n    println(\"Hello ${name} ${not false}\");\n}"
        );
    }
}
