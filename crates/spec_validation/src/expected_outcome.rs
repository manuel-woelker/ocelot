use ocelot_base::shared_string::SharedString;

/// Expected observable result for one spec example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedOutcome {
    Output(SharedString),
    Error(SharedString),
}

impl ExpectedOutcome {
    /// Returns the expected comparison text regardless of outcome kind.
    pub fn text(&self) -> &str {
        match self {
            Self::Output(text) | Self::Error(text) => text.as_str(),
        }
    }
}
