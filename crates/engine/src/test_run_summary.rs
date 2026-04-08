use crate::failed_test_result::FailedTestResult;
use crate::passed_test_result::PassedTestResult;

/// Summary of one `ocelot test` file execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRunSummary {
    pub passed: Vec<PassedTestResult>,
    pub failed: Vec<FailedTestResult>,
}

impl TestRunSummary {
    /// Creates an empty summary.
    pub fn new() -> Self {
        Self {
            passed: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// Returns true when every discovered test passed.
    pub fn is_success(&self) -> bool {
        self.failed.is_empty()
    }
}

impl Default for TestRunSummary {
    fn default() -> Self {
        Self::new()
    }
}
