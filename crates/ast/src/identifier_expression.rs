/// Identifier expression payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierExpression {
    pub name: String,
}

impl IdentifierExpression {
    /// Creates an identifier expression payload.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
