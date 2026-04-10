use ocelot_base::shared_string::SharedString;

/// Observed result of executing one spec example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservedOutcome {
    Output(SharedString),
    Error(SharedString),
    EvalResult(SharedString),
}

impl ObservedOutcome {
    /// Returns the observed comparison text regardless of outcome kind.
    pub fn text(&self) -> &str {
        match self {
            Self::Output(text) | Self::Error(text) | Self::EvalResult(text) => text.as_str(),
        }
    }
}
