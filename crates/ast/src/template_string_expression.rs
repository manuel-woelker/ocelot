use crate::template_string_part::TemplateStringPart;

/// Template string expression payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateStringExpression {
    pub parts: Vec<TemplateStringPart>,
}

impl TemplateStringExpression {
    /// Creates a template string expression payload.
    pub fn new(parts: Vec<TemplateStringPart>) -> Self {
        Self { parts }
    }
}
