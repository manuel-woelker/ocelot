use ocelot_base::shared_string::SharedString;

/// Failure details for one executed test item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedTestResult {
    pub name: SharedString,
    pub message: SharedString,
}

impl FailedTestResult {
    /// Creates a failed test result from its user-facing name and message.
    pub fn new(name: impl Into<SharedString>, message: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            message: message.into(),
        }
    }
}
