use crate::expected_outcome::ExpectedOutcome;
use crate::spec_example_file::SpecExampleFile;
use ocelot_base::file_path::FilePath;
use ocelot_base::shared_string::SharedString;

/// Executable example extracted from a spec chapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecExample {
    /// The spec chapter that defined this example.
    pub chapter_path: FilePath,
    /// The visible example heading text without the `## Example:` prefix.
    pub name: SharedString,
    /// The named source files declared by this example.
    pub source_files: Vec<SpecExampleFile>,
    /// The expected normalized result from the fenced `text` block.
    pub expected_outcome: ExpectedOutcome,
    /// The one-based line number of the example heading in the chapter.
    pub line_number: usize,
}
