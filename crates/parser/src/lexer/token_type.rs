/// Token kinds needed for the first script-style `println()` programs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Can,
    Cannot,
    Effect,
    Identifier,
    String,
    False,
    Fun,
    Not,
    True,
    Use,
    Comma,
    DoubleColon,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Semicolon,
    Test,
    Unexpected,
    EndOfFile,
}
