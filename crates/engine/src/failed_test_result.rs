use ocelot_base::file_path::FilePath;
use ocelot_base::shared_string::SharedString;

/// Failure details for one executed test item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedTestResult {
    pub file_path: FilePath,
    pub name: SharedString,
    pub message: SharedString,
}

impl FailedTestResult {
    /// Creates a failed test result from its source file path, user-facing name, and message.
    pub fn new(
        file_path: impl Into<FilePath>,
        name: impl Into<SharedString>,
        message: impl Into<SharedString>,
    ) -> Self {
        Self {
            file_path: file_path.into(),
            name: name.into(),
            message: message.into(),
        }
    }
}
