use crate::lexer::token::Token;
use crate::lexer::token_type::TokenType;
use ocelot_base::compilation_context::CompilationContext;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;

/// Tokenizes a source file into lexical tokens and appends diagnostics to the context.
pub fn lex(source_file: &SourceFile, context: &mut CompilationContext) -> Vec<Token> {
    let mut tokens = Vec::new();
    let source = source_file.source();
    let bytes = source.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b' ' | b'\t' | b'\n' | b'\r' => {
                index += 1;
            }
            b'(' => {
                tokens.push(Token::new(TokenType::LeftParen, index, index + 1));
                index += 1;
            }
            b',' => {
                tokens.push(Token::new(TokenType::Comma, index, index + 1));
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
                    context.add_diagnostic(unterminated_string_diagnostic(
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
                    "test" => TokenType::Test,
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
    let (line_number, line_start, line_end) = line_bounds(source_file.source(), start);
    let excerpt = SourceExcerpt::new(
        &source_file.path,
        line_number,
        &source_file.source()[line_start..line_end],
    )
    .with_annotation(SourceAnnotation::new(
        Span::new(start - line_start, end - line_start),
        "string is missing a closing quote",
    ));

    SourceDiagnostic::new(
        DiagnosticLevel::Error,
        &source_file.path,
        "unterminated string literal",
    )
    .with_excerpt(excerpt)
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
    use ocelot_base::compilation_context::CompilationContext;
    use ocelot_base::diagnostic_level::DiagnosticLevel;
    use ocelot_base::source_file::SourceFile;

    #[test]
    fn lexes_println_script() {
        let source_file = SourceFile::new("examples/hello.ocelot", "println(\"hello\");");
        let mut context = CompilationContext::default();
        let token_types: Vec<_> = lex(&source_file, &mut context)
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
        assert!(!context.has_errors());
    }

    #[test]
    fn lexes_test_item_tokens() {
        let source_file = SourceFile::new(
            "examples/tests.ocelot",
            "test \"prints\" { println(\"hello\"); }",
        );
        let mut context = CompilationContext::default();
        let token_types: Vec<_> = lex(&source_file, &mut context)
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
        assert!(!context.has_errors());
    }

    #[test]
    fn skips_whitespace_between_tokens() {
        let source_file = SourceFile::new("examples/whitespace.ocelot", "println ( \"hello\" ) ;");
        let mut context = CompilationContext::default();
        let token_types: Vec<_> = lex(&source_file, &mut context)
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
        assert!(!context.has_errors());
    }

    #[test]
    fn lexes_commas_in_argument_lists() {
        let source_file = SourceFile::new(
            "examples/arguments.ocelot",
            "println(\"hello\", \"world\");",
        );
        let mut context = CompilationContext::default();
        let token_types: Vec<_> = lex(&source_file, &mut context)
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
        assert!(!context.has_errors());
    }

    #[test]
    fn reports_unterminated_strings_as_source_diagnostics() {
        let source_file = SourceFile::new("examples/broken.ocelot", "println(\"hello);");
        let mut context = CompilationContext::default();
        let token_types: Vec<_> = lex(&source_file, &mut context)
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
        assert!(context.has_errors());
        assert_eq!(context.source_diagnostics.diagnostics.len(), 1);

        let diagnostic = &context.source_diagnostics.diagnostics[0];

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
        let mut context = CompilationContext::default();
        let token_types: Vec<_> = lex(&source_file, &mut context)
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
        assert!(context.has_errors());
        assert_eq!(context.source_diagnostics.diagnostics.len(), 1);

        let diagnostic = &context.source_diagnostics.diagnostics[0];

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
    fn emits_unexpected_tokens_for_unknown_characters() {
        let source_file = SourceFile::new("examples/unexpected.ocelot", "@");
        let mut context = CompilationContext::default();
        let token_types: Vec<_> = lex(&source_file, &mut context)
            .into_iter()
            .map(|token| token.token_type)
            .collect();

        assert_eq!(
            token_types,
            vec![TokenType::Unexpected, TokenType::EndOfFile]
        );
        assert!(!context.has_errors());
    }
}
