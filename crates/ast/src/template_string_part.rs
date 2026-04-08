use crate::expression::Expression;
use ocelot_base::shared_string::SharedString;

/// One ordered part of a template string expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateStringPart {
    Interpolation(Expression),
    Text(SharedString),
}
