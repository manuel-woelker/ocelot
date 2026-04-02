use crate::source_diagnostic::SourceDiagnostic;

/// Collection of structured source diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceDiagnostics {
    /// Diagnostics included in this collection.
    pub diagnostics: Vec<SourceDiagnostic>,
}

impl SourceDiagnostics {
    /// Creates an empty diagnostic collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a copy of this collection with one appended diagnostic.
    pub fn with_diagnostic(mut self, diagnostic: SourceDiagnostic) -> Self {
        self.diagnostics.push(diagnostic);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::SourceDiagnostics;
    use crate::diagnostic_level::DiagnosticLevel;
    use crate::source_diagnostic::SourceDiagnostic;

    #[test]
    fn source_diagnostics_tracks_multiple_entries() {
        let diagnostics = SourceDiagnostics::new()
            .with_diagnostic(SourceDiagnostic::new(
                DiagnosticLevel::Error,
                "examples/test.ocelot",
                "first error",
            ))
            .with_diagnostic(SourceDiagnostic::new(
                DiagnosticLevel::Warning,
                "examples/test.ocelot",
                "second warning",
            ));

        assert_eq!(diagnostics.diagnostics.len(), 2);
        assert_eq!(diagnostics.diagnostics[0].message, "first error");
        assert_eq!(diagnostics.diagnostics[1].message, "second warning");
    }
}
