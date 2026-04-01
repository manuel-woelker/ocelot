/// String literal expression payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteralExpression {
    pub value: String,
}

impl StringLiteralExpression {
    /// Creates a string literal expression payload.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}
