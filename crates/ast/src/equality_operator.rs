/// Comparison performed by an equality expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqualityOperator {
    /// Tests whether two values are equal.
    Equal,
    /// Tests whether two values differ.
    NotEqual,
}

impl EqualityOperator {
    /// Returns the operator's source spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "==",
            Self::NotEqual => "!=",
        }
    }
}
