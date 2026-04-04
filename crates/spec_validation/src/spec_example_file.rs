use ocelot_base::file_path::FilePath;
use ocelot_base::shared_string::SharedString;

/// One source file declared by a spec example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecExampleFile {
    pub path: FilePath,
    pub source: SharedString,
}

impl SpecExampleFile {
    /// Creates a named source file for one spec example.
    pub fn new(path: impl Into<FilePath>, source: impl Into<SharedString>) -> Self {
        Self {
            path: path.into(),
            source: source.into(),
        }
    }
}
