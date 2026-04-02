use ocelot_base::file_path::FilePath;
use ocelot_base::shared_string::SharedString;

/// Executable example extracted from a spec chapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecExample {
    /// The spec chapter that defined this example.
    pub chapter_path: FilePath,
    /// The visible example heading text without the `## Example:` prefix.
    pub name: SharedString,
    /// The source code from the fenced `ocelot` block.
    pub source: SharedString,
    /// The expected normalized output from the fenced `text` block.
    pub expected_output: SharedString,
    /// The one-based line number of the example heading in the chapter.
    pub line_number: usize,
}
