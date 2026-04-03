use crate::diagnostic_level::DiagnosticLevel;
use crate::source_diagnostics::SourceDiagnostics;

/// Compilation state that accumulates source diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompilationContext {
    /// Diagnostics produced during compilation.
    pub source_diagnostics: SourceDiagnostics,
}

impl CompilationContext {
    /// Creates a compilation context from the provided source diagnostics.
    pub fn new(source_diagnostics: SourceDiagnostics) -> Self {
        Self { source_diagnostics }
    }

    /// Returns true when any tracked diagnostic has error severity.
    pub fn has_errors(&self) -> bool {
        self.source_diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::CompilationContext;
    use crate::diagnostic_level::DiagnosticLevel;
    use crate::source_diagnostic::SourceDiagnostic;
    use crate::source_diagnostics::SourceDiagnostics;

    #[test]
    fn compilation_context_reports_errors_when_present() {
        let context = CompilationContext::new(
            SourceDiagnostics::new()
                .with_diagnostic(SourceDiagnostic::new(
                    DiagnosticLevel::Warning,
                    "examples/test.ocelot",
                    "unused value",
                ))
                .with_diagnostic(SourceDiagnostic::new(
                    DiagnosticLevel::Error,
                    "examples/test.ocelot",
                    "type mismatch",
                )),
        );

        assert!(context.has_errors());
    }

    #[test]
    fn compilation_context_reports_no_errors_when_only_warnings_are_present() {
        let context = CompilationContext::new(SourceDiagnostics::new().with_diagnostic(
            SourceDiagnostic::new(
                DiagnosticLevel::Warning,
                "examples/test.ocelot",
                "unused value",
            ),
        ));

        assert!(!context.has_errors());
    }
}
