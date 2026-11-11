/// A decimal magnitude; 2^63 is valid only directly inside unary negation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerLiteralExpression {
    /// Parser-checked magnitude, normally at most i64::MAX.
    pub magnitude: u64,
}
