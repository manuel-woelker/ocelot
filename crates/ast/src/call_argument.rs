use crate::expression::Expression;
use crate::identifier::Identifier;

/// One call-site argument with an optional explicit parameter label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallArgument {
    /// The label written at the call site, when present.
    pub label: Option<Identifier>,
    /// The expression evaluated to supply this argument's value.
    pub value: Expression,
}

impl CallArgument {
    /// Creates an argument with an optional explicit label.
    pub fn new(label: Option<Identifier>, value: Expression) -> Self {
        Self { label, value }
    }
}
