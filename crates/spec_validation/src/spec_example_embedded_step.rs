use crate::spec_example_embedded_step_kind::SpecExampleEmbeddedStepKind;
use ocelot_base::shared_string::SharedString;

/// One embedded-engine action declared inside a spec example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecExampleEmbeddedStep {
    pub kind: SpecExampleEmbeddedStepKind,
    pub source: SharedString,
}

impl SpecExampleEmbeddedStep {
    /// Creates one embedded-engine step from its kind and source text.
    pub fn new(kind: SpecExampleEmbeddedStepKind, source: impl Into<SharedString>) -> Self {
        Self {
            kind,
            source: source.into(),
        }
    }
}
