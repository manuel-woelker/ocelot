use ocelot_base::file_path::FilePath;
use ocelot_base::shared_string::SharedString;
use ocelot_base::span::Span;

/// Metadata describing one discovered language-level test item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredTest {
    pub file_path: FilePath,
    pub name: SharedString,
    pub span: Span,
}

impl DiscoveredTest {
    /// Creates a discovered test result.
    pub fn new(file_path: impl Into<FilePath>, name: impl Into<SharedString>, span: Span) -> Self {
        Self {
            file_path: file_path.into(),
            name: name.into(),
            span,
        }
    }
}
