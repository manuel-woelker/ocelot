use ocelot_base::span::Span;

/// Token kinds needed for the first script-style `println()` programs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Identifier,
    String,
    LeftParen,
    RightParen,
    Semicolon,
    EndOfFile,
}

/// Lexical token with its source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    pub span: Span,
}

impl Token {
    /// Creates a token from its kind and byte offsets.
    pub const fn new(token_type: TokenType, start: usize, end: usize) -> Self {
        Self {
            token_type,
            span: Span::new(start, end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Token, TokenType};

    #[test]
    fn token_captures_kind_and_span() {
        let token = Token::new(TokenType::Identifier, 0, 7);

        assert_eq!(token.token_type, TokenType::Identifier);
        assert_eq!(token.span.start(), 0);
        assert_eq!(token.span.end(), 7);
    }

    #[test]
    fn println_script_tokens_are_representable() {
        let tokens = [
            Token::new(TokenType::Identifier, 0, 7),
            Token::new(TokenType::LeftParen, 7, 8),
            Token::new(TokenType::String, 8, 15),
            Token::new(TokenType::RightParen, 15, 16),
            Token::new(TokenType::Semicolon, 16, 17),
            Token::new(TokenType::EndOfFile, 17, 17),
        ];

        assert_eq!(tokens[0].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].token_type, TokenType::LeftParen);
        assert_eq!(tokens[2].token_type, TokenType::String);
        assert_eq!(tokens[3].token_type, TokenType::RightParen);
        assert_eq!(tokens[4].token_type, TokenType::Semicolon);
        assert_eq!(tokens[5].token_type, TokenType::EndOfFile);
    }
}
