use crate::println_statement::PrintlnStatement;

/// Variants of top-level script statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementKind {
    Println(PrintlnStatement),
}
