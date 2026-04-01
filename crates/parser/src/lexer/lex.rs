use crate::lexer::token::Token;
use crate::lexer::token_type::TokenType;
use ocelot_base::result::OcelotResult;

/// Tokenizes source text into an iterator of tokens.
pub fn lex(source: &str) -> OcelotResult<impl Iterator<Item = Token>> {
    let mut tokens = Vec::new();
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
            b')' => {
                tokens.push(Token::new(TokenType::RightParen, index, index + 1));
                index += 1;
            }
            b';' => {
                tokens.push(Token::new(TokenType::Semicolon, index, index + 1));
                index += 1;
            }
            b'"' => {
                let start = index;
                index += 1;

                while index < bytes.len() && bytes[index] != b'"' {
                    index += 1;
                }

                if index >= bytes.len() {
                    ocelot_base::bail!("unterminated string literal");
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

                tokens.push(Token::new(TokenType::Identifier, start, index));
            }
            _ => {
                tokens.push(Token::new(TokenType::Unexpected, index, index + 1));
                index += 1;
            }
        }
    }

    tokens.push(Token::new(TokenType::EndOfFile, index, index));
    Ok(tokens.into_iter())
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_continue(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::lex;
    use crate::lexer::token_type::TokenType;

    #[test]
    fn lexes_println_script() {
        let token_types: Vec<_> = lex("println(\"hello\");")
            .unwrap()
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
    }

    #[test]
    fn skips_whitespace_between_tokens() {
        let token_types: Vec<_> = lex("println ( \"hello\" ) ;")
            .unwrap()
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
    }

    #[test]
    fn rejects_unterminated_strings() {
        let error = lex("println(\"hello);").err().unwrap();

        assert!(
            error
                .to_test_string()
                .contains("unterminated string literal")
        );
    }

    #[test]
    fn emits_unexpected_tokens_for_unknown_characters() {
        let token_types: Vec<_> = lex("@").unwrap().map(|token| token.token_type).collect();

        assert_eq!(
            token_types,
            vec![TokenType::Unexpected, TokenType::EndOfFile]
        );
    }
}
