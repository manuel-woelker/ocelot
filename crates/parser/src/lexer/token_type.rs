/// Token kinds needed for the first script-style `println()` programs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Identifier,
    String,
    LeftParen,
    RightParen,
    Semicolon,
    Unexpected,
    EndOfFile,
}
