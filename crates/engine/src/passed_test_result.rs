use ocelot_base::file_path::FilePath;
use ocelot_base::shared_string::SharedString;

/// Success details for one executed test item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassedTestResult {
    pub file_path: FilePath,
    pub name: SharedString,
}

impl PassedTestResult {
    /// Creates a passed test result from its source file path and user-facing name.
    pub fn new(file_path: impl Into<FilePath>, name: impl Into<SharedString>) -> Self {
        Self {
            file_path: file_path.into(),
            name: name.into(),
        }
    }
}
