use crate::validation_failure::ValidationFailure;

/// Overall result produced by validating spec examples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    /// The number of numbered spec chapters scanned.
    pub scanned_chapter_count: usize,
    /// The total number of executable examples discovered.
    pub example_count: usize,
    /// The number of examples whose observed output matched.
    pub passed_example_count: usize,
    /// The failures found while extracting or executing examples.
    pub failures: Vec<ValidationFailure>,
}

impl ValidationReport {
    /// Returns `true` when validation completed without failures.
    pub fn is_success(&self) -> bool {
        self.failures.is_empty()
    }
}
