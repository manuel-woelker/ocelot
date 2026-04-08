use ocelot_base::shared_string::SharedString;

/// Filter parts used to match tests by name or file path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestFilter {
    parts: Vec<SharedString>,
}

impl TestFilter {
    /// Creates a test filter from already-normalized parts.
    pub fn new(parts: Vec<SharedString>) -> Self {
        Self { parts }
    }

    /// Returns true when no filter parts are present.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Returns the normalized filter parts.
    pub fn parts(&self) -> &[SharedString] {
        &self.parts
    }
}
