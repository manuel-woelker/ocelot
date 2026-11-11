/// Boolean operators with explicit grouping rather than relative precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOperator {
    /// True only when both operands are true.
    And,
    /// True when either operand is true.
    Or,
}

impl LogicalOperator {
    /// Returns the source spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
        }
    }

    /// Returns the left value that makes evaluating the right unnecessary.
    pub fn short_circuits_on(self) -> bool {
        self == Self::Or
    }
}
