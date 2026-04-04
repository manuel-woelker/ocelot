/// Boolean literal expression payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BooleanLiteralExpression {
    pub value: bool,
}

impl BooleanLiteralExpression {
    /// Creates a boolean literal expression payload.
    pub const fn new(value: bool) -> Self {
        Self { value }
    }
}
