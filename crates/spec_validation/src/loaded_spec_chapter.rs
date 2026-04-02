use crate::spec_example::SpecExample;
use crate::validation_failure::ValidationFailure;
use ocelot_base::file_path::FilePath;

/// Loaded data from one spec markdown chapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSpecChapter {
    /// The chapter path relative to the repository root.
    pub path: FilePath,
    /// The executable examples extracted from the chapter.
    pub examples: Vec<SpecExample>,
    /// The malformed example failures found while parsing the chapter.
    pub malformed_failures: Vec<ValidationFailure>,
}
