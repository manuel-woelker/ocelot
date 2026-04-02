use crate::validation_failure_kind::ValidationFailureKind;
use ocelot_base::file_path::FilePath;
use ocelot_base::shared_string::SharedString;

/// What single validation failure should be reported to the caller?
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFailure {
    /// The chapter path that contains the failing example.
    pub chapter_path: FilePath,
    /// The example heading text, or a synthetic label for malformed content.
    pub example_name: SharedString,
    /// The category of validation failure.
    pub kind: ValidationFailureKind,
    /// A short human-readable explanation of the failure.
    pub message: SharedString,
    /// The expected normalized output when one was available.
    pub expected_output: Option<SharedString>,
    /// The observed normalized output when one was available.
    pub actual_output: Option<SharedString>,
    /// The one-based line number associated with the failure.
    pub line_number: usize,
}
