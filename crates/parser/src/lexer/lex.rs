use crate::lexer::token::Token;
use crate::lexer::token_type::TokenType;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;

/// Tokenizes a source file into lexical tokens and appends diagnostics to the source_diagnostics.
pub fn lex(source_file: &SourceFile, source_diagnostics: &mut SourceDiagnostics) -> Vec<Token> {
    let mut tokens = Vec::new();
    let source = source_file.source();
    let bytes = source.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        match skip_trivia(source_file, index) {
            Ok(next_index) => index = next_index,
            Err(diagnostic) => {
                source_diagnostics.add_diagnostic(diagnostic);
                index = bytes.len();
                break;
            }
        }

        if index >= bytes.len() {
            break;
        }

        match bytes[index] {
            b'(' => {
                tokens.push(Token::new(TokenType::LeftParen, index, index + 1));
                index += 1;
            }
            b',' => {
                tokens.push(Token::new(TokenType::Comma, index, index + 1));
                index += 1;
            }
            b':' if index + 1 < bytes.len() && bytes[index + 1] == b':' => {
                tokens.push(Token::new(TokenType::DoubleColon, index, index + 2));
                index += 2;
            }
            b':' => {
                tokens.push(Token::new(TokenType::Colon, index, index + 1));
                index += 1;
            }
            b')' => {
                tokens.push(Token::new(TokenType::RightParen, index, index + 1));
                index += 1;
            }
            b'{' => {
                tokens.push(Token::new(TokenType::LeftBrace, index, index + 1));
                index += 1;
            }
            b'}' => {
                tokens.push(Token::new(TokenType::RightBrace, index, index + 1));
                index += 1;
            }
            b';' => {
                tokens.push(Token::new(TokenType::Semicolon, index, index + 1));
                index += 1;
            }
            b'"' => {
                let start = index;
                index += 1;

                while index < bytes.len()
                    && bytes[index] != b'"'
                    && bytes[index] != b'\n'
                    && bytes[index] != b'\r'
                {
                    index += 1;
                }

                if index >= bytes.len() || bytes[index] == b'\n' || bytes[index] == b'\r' {
                    source_diagnostics.add_diagnostic(unterminated_string_diagnostic(
                        source_file,
                        start,
                        index,
                    ));
                    break;
                }

                index += 1;
                tokens.push(Token::new(TokenType::String, start, index));
            }
            byte if is_identifier_start(byte) => {
                let start = index;
                index += 1;

                while index < bytes.len() && is_identifier_continue(bytes[index]) {
                    index += 1;
                }

                let token_type = match &source[start..index] {
                    "can" => TokenType::Can,
                    "cannot" => TokenType::Cannot,
                    "effect" => TokenType::Effect,
                    "false" => TokenType::False,
                    "fun" => TokenType::Fun,
                    "native" => TokenType::Native,
                    "not" => TokenType::Not,
                    "test" => TokenType::Test,
                    "true" => TokenType::True,
                    "use" => TokenType::Use,
                    _ => TokenType::Identifier,
                };
                tokens.push(Token::new(token_type, start, index));
            }
            _ => {
                tokens.push(Token::new(TokenType::Unexpected, index, index + 1));
                index += 1;
            }
        }
    }

    tokens.push(Token::new(TokenType::EndOfFile, index, index));
    tokens
}

fn skip_trivia(source_file: &SourceFile, mut index: usize) -> Result<usize, SourceDiagnostic> {
    let bytes = source_file.source().as_bytes();

    loop {
        if index >= bytes.len() {
            return Ok(index);
        }

        match bytes[index] {
            b' ' | b'\t' | b'\n' | b'\r' => {
                index += 1;
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index += 2;

                while index < bytes.len() && bytes[index] != b'\n' && bytes[index] != b'\r' {
                    index += 1;
                }
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index = skip_block_comment(source_file, index)?;
            }
            _ => return Ok(index),
        }
    }
}

fn skip_block_comment(source_file: &SourceFile, start: usize) -> Result<usize, SourceDiagnostic> {
    let bytes = source_file.source().as_bytes();
    let mut index = start + 2;
    let mut depth = 1usize;

    while index < bytes.len() {
        if index + 1 < bytes.len() && bytes[index] == b'/' && bytes[index + 1] == b'*' {
            depth += 1;
            index += 2;
            continue;
        }

        if index + 1 < bytes.len() && bytes[index] == b'*' && bytes[index + 1] == b'/' {
            depth -= 1;
            index += 2;

            if depth == 0 {
                return Ok(index);
            }

            continue;
        }

        index += 1;
    }

    Err(unterminated_block_comment_diagnostic(
        source_file,
        start,
        source_file.source().len(),
    ))
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_continue(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit()
}

fn unterminated_string_diagnostic(
    source_file: &SourceFile,
    start: usize,
    end: usize,
) -> SourceDiagnostic {
    let excerpt =
        excerpt_with_annotation(source_file, start, end, "string is missing a closing quote");

    SourceDiagnostic::new(
        DiagnosticLevel::Error,
        &source_file.path,
        "unterminated string literal",
    )
    .with_excerpt(excerpt)
}

fn unterminated_block_comment_diagnostic(
    source_file: &SourceFile,
    start: usize,
    end: usize,
) -> SourceDiagnostic {
    let excerpt = excerpt_with_annotation(
        source_file,
        start,
        end,
        "block comment is missing a closing `*/`",
    );

    SourceDiagnostic::new(
        DiagnosticLevel::Error,
        &source_file.path,
        "unterminated block comment",
    )
    .with_excerpt(excerpt)
}

fn excerpt_with_annotation(
    source_file: &SourceFile,
    start: usize,
    end: usize,
    annotation_message: &str,
) -> SourceExcerpt {
    let (line_number, line_start, line_end) = line_bounds(source_file.source(), start);
    let annotation_end = end.min(line_end).max(start + 1);

    SourceExcerpt::new(
        &source_file.path,
        line_number,
        &source_file.source()[line_start..line_end],
    )
    .with_annotation(SourceAnnotation::new(
        Span::new(start - line_start, annotation_end - line_start),
        annotation_message,
    ))
}

fn line_bounds(source: &str, index: usize) -> (usize, usize, usize) {
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

#[cfg(test)]
mod tests {
    use super::lex;
    use crate::lexer::token_type::TokenType;
    use ocelot_base::diagnostic_level::DiagnosticLevel;
    use ocelot_base::source_diagnostics::SourceDiagnostics;
    use ocelot_base::source_file::SourceFile;

    #[test]
    fn lexes_println_script() {
        let source_file = SourceFile::new("examples/hello.ocelot", "println(\"hello\");");
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::String,
                TokenType::RightParen,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn lexes_test_item_tokens() {
        let source_file = SourceFile::new(
            "examples/tests.ocelot",
            "test \"prints\" { println(\"hello\"); }",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Test,
                TokenType::String,
                TokenType::LeftBrace,
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::String,
                TokenType::RightParen,
                TokenType::Semicolon,
                TokenType::RightBrace,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn lexes_function_item_tokens() {
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun greet() { println(\"hello\"); }",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Fun,
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::RightParen,
                TokenType::LeftBrace,
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::String,
                TokenType::RightParen,
                TokenType::Semicolon,
                TokenType::RightBrace,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn lexes_colons_in_function_parameter_lists() {
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun greet(name: string, excited: bool) {}",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Fun,
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::Identifier,
                TokenType::Colon,
                TokenType::Identifier,
                TokenType::Comma,
                TokenType::Identifier,
                TokenType::Colon,
                TokenType::Identifier,
                TokenType::RightParen,
                TokenType::LeftBrace,
                TokenType::RightBrace,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn lexes_effect_keywords() {
        let source_file = SourceFile::new(
            "examples/effects.ocelot",
            "effect exec; fun greet() can exec cannot panic {}",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Effect,
                TokenType::Identifier,
                TokenType::Semicolon,
                TokenType::Fun,
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::RightParen,
                TokenType::Can,
                TokenType::Identifier,
                TokenType::Cannot,
                TokenType::Identifier,
                TokenType::LeftBrace,
                TokenType::RightBrace,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn lexes_boolean_literals_as_reserved_tokens() {
        let source_file = SourceFile::new("examples/booleans.ocelot", "true; false;");
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::True,
                TokenType::Semicolon,
                TokenType::False,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn lexes_not_as_a_reserved_token() {
        let source_file = SourceFile::new("examples/not.ocelot", "not true;");
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Not,
                TokenType::True,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn keeps_longer_identifiers_distinct_from_boolean_literals() {
        let source_file = SourceFile::new("examples/booleans.ocelot", "true_value; falsey;");
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::Semicolon,
                TokenType::Identifier,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn keeps_longer_identifiers_distinct_from_not() {
        let source_file = SourceFile::new("examples/not.ocelot", "notify; not_value; knot;");
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::Semicolon,
                TokenType::Identifier,
                TokenType::Semicolon,
                TokenType::Identifier,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn skips_whitespace_between_tokens() {
        let source_file = SourceFile::new("examples/whitespace.ocelot", "println ( \"hello\" ) ;");
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::String,
                TokenType::RightParen,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn skips_line_comments_between_tokens() {
        let source_file = SourceFile::new(
            "examples/comments.ocelot",
            "// heading comment\nprintln(\"hello\"); // trailing comment",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::String,
                TokenType::RightParen,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn skips_block_comments_between_tokens() {
        let source_file = SourceFile::new(
            "examples/comments.ocelot",
            "println/* call */(\"hello\"/* value */);",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::String,
                TokenType::RightParen,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn skips_nested_block_comments() {
        let source_file = SourceFile::new(
            "examples/comments.ocelot",
            "/* outer /* inner */ still outer */ println(\"hello\");",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::String,
                TokenType::RightParen,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn skips_multiline_block_comments() {
        let source_file = SourceFile::new(
            "examples/comments.ocelot",
            "/* setup\n   before call */\nprintln(\"hello\");",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::String,
                TokenType::RightParen,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn keeps_comment_markers_inside_strings() {
        let source_file = SourceFile::new(
            "examples/comments.ocelot",
            "println(\"// not a comment /* still text */\");",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let tokens = lex(&source_file, &mut source_diagnostics);

        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[2].token_type, TokenType::String);
        assert_eq!(
            &source_file.source()[tokens[2].span.start()..tokens[2].span.end()],
            "\"// not a comment /* still text */\""
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn lexes_commas_in_argument_lists() {
        let source_file = SourceFile::new(
            "examples/arguments.ocelot",
            "println(\"hello\", \"world\");",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::String,
                TokenType::Comma,
                TokenType::String,
                TokenType::RightParen,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn reports_unterminated_strings_as_source_diagnostics() {
        let source_file = SourceFile::new("examples/broken.ocelot", "println(\"hello);");
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::EndOfFile
            ]
        );
        assert!(source_diagnostics.has_errors());
        assert_eq!(source_diagnostics.diagnostics.len(), 1);

        let diagnostic = &source_diagnostics.diagnostics[0];

        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert_eq!(diagnostic.file_path.as_str(), "examples/broken.ocelot");
        assert_eq!(diagnostic.message, "unterminated string literal");
        assert_eq!(diagnostic.excerpts.len(), 1);
        assert_eq!(diagnostic.excerpts[0].line_number, 1);
        assert_eq!(diagnostic.excerpts[0].source_line, "println(\"hello);");
        assert_eq!(diagnostic.excerpts[0].annotations.len(), 1);
        assert_eq!(
            diagnostic.excerpts[0].annotations[0].span,
            ocelot_base::span::Span::new(8, 16)
        );
        assert_eq!(
            diagnostic.excerpts[0].annotations[0].message,
            "string is missing a closing quote"
        );
    }

    #[test]
    fn reports_strings_terminated_by_a_newline_as_source_diagnostics() {
        let source_file = SourceFile::new("examples/broken.ocelot", "println(\"hello);\n");
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::EndOfFile
            ]
        );
        assert!(source_diagnostics.has_errors());
        assert_eq!(source_diagnostics.diagnostics.len(), 1);

        let diagnostic = &source_diagnostics.diagnostics[0];

        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert_eq!(diagnostic.file_path.as_str(), "examples/broken.ocelot");
        assert_eq!(diagnostic.message, "unterminated string literal");
        assert_eq!(diagnostic.excerpts.len(), 1);
        assert_eq!(diagnostic.excerpts[0].line_number, 1);
        assert_eq!(diagnostic.excerpts[0].source_line, "println(\"hello);");
        assert_eq!(diagnostic.excerpts[0].annotations.len(), 1);
        assert_eq!(
            diagnostic.excerpts[0].annotations[0].span,
            ocelot_base::span::Span::new(8, 16)
        );
        assert_eq!(
            diagnostic.excerpts[0].annotations[0].message,
            "string is missing a closing quote"
        );
    }

    #[test]
    fn reports_unterminated_block_comments_as_source_diagnostics() {
        let source_file = SourceFile::new("examples/broken.ocelot", "println(/* hello");
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::EndOfFile
            ]
        );
        assert!(source_diagnostics.has_errors());
        assert_eq!(source_diagnostics.diagnostics.len(), 1);

        let diagnostic = &source_diagnostics.diagnostics[0];

        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert_eq!(diagnostic.file_path.as_str(), "examples/broken.ocelot");
        assert_eq!(diagnostic.message, "unterminated block comment");
        assert_eq!(diagnostic.excerpts.len(), 1);
        assert_eq!(diagnostic.excerpts[0].line_number, 1);
        assert_eq!(diagnostic.excerpts[0].source_line, "println(/* hello");
        assert_eq!(diagnostic.excerpts[0].annotations.len(), 1);
        assert_eq!(
            diagnostic.excerpts[0].annotations[0].span,
            ocelot_base::span::Span::new(8, 16)
        );
        assert_eq!(
            diagnostic.excerpts[0].annotations[0].message,
            "block comment is missing a closing `*/`"
        );
    }

    #[test]
    fn reports_unterminated_multiline_block_comments_from_the_opening_line() {
        let source_file = SourceFile::new("examples/broken.ocelot", "/* hello\nprintln(\"x\");");
        let mut source_diagnostics = SourceDiagnostics::default();

        lex(&source_file, &mut source_diagnostics);

        assert!(source_diagnostics.has_errors());
        assert_eq!(source_diagnostics.diagnostics.len(), 1);

        let diagnostic = &source_diagnostics.diagnostics[0];

        assert_eq!(diagnostic.message, "unterminated block comment");
        assert_eq!(diagnostic.excerpts[0].line_number, 1);
        assert_eq!(diagnostic.excerpts[0].source_line, "/* hello");
        assert_eq!(
            diagnostic.excerpts[0].annotations[0].span,
            ocelot_base::span::Span::new(0, 8)
        );
    }

    #[test]
    fn emits_unexpected_tokens_for_unknown_characters() {
        let source_file = SourceFile::new("examples/unexpected.ocelot", "@");
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![TokenType::Unexpected, TokenType::EndOfFile]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn lexes_double_colon_qualified_identifiers() {
        let source_file = SourceFile::new("examples/module.ocelot", "math::greet::hello();");
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Identifier,
                TokenType::DoubleColon,
                TokenType::Identifier,
                TokenType::DoubleColon,
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::RightParen,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
    }

    #[test]
    fn lexes_use_items_and_grouped_imports() {
        let source_file = SourceFile::new(
            "examples/imports.ocelot-script",
            "use math::trig::{sin, cos};",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Use,
                TokenType::Identifier,
                TokenType::DoubleColon,
                TokenType::Identifier,
                TokenType::DoubleColon,
                TokenType::LeftBrace,
                TokenType::Identifier,
                TokenType::Comma,
                TokenType::Identifier,
                TokenType::RightBrace,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }

    #[test]
    fn lexes_native_function_declarations() {
        let source_file = SourceFile::new(
            "examples/core.ocelot",
            "native fun println(value: any) can write_stdout;",
        );
        let mut source_diagnostics = SourceDiagnostics::default();
        let token_types: Vec<_> = lex(&source_file, &mut source_diagnostics)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![
                TokenType::Native,
                TokenType::Fun,
                TokenType::Identifier,
                TokenType::LeftParen,
                TokenType::Identifier,
                TokenType::Colon,
                TokenType::Identifier,
                TokenType::RightParen,
                TokenType::Can,
                TokenType::Identifier,
                TokenType::Semicolon,
                TokenType::EndOfFile,
            ]
        );
        assert!(!source_diagnostics.has_errors());
    }
}
