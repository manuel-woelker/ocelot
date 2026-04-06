use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_diagnostics::SourceDiagnostics;

/// Compilation state that accumulates source diagnostics.
#[derive(Debug, Clone, Default)]
pub struct CompilationContext {
    /// Diagnostics produced during compilation.
    pub source_diagnostics: SourceDiagnostics,
}

impl CompilationContext {
    /// Creates a compilation context from the provided source diagnostics.
    pub fn new(source_diagnostics: SourceDiagnostics) -> Self {
        Self { source_diagnostics }
    }

    /// Appends one diagnostic to this compilation context.
    pub fn add_diagnostic(&mut self, diagnostic: SourceDiagnostic) {
        self.source_diagnostics.diagnostics.push(diagnostic);
    }

    /// Appends multiple diagnostics to this compilation context.
    pub fn add_diagnostics(&mut self, diagnostics: impl IntoIterator<Item = SourceDiagnostic>) {
        self.source_diagnostics.diagnostics.extend(diagnostics);
    }

    /// Returns true when any tracked diagnostic has error severity.
    pub fn has_errors(&self) -> bool {
        self.source_diagnostics.has_errors()
    }
}

#[cfg(test)]
mod tests {
    use super::CompilationContext;
    use ocelot_base::diagnostic_level::DiagnosticLevel;
    use ocelot_base::source_diagnostic::SourceDiagnostic;
    use ocelot_base::source_diagnostics::SourceDiagnostics;

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

    #[test]
    fn compilation_context_can_add_one_diagnostic() {
        let mut context = CompilationContext::default();

        context.add_diagnostic(SourceDiagnostic::new(
            DiagnosticLevel::Warning,
            "examples/test.ocelot",
            "unused value",
        ));

        assert_eq!(context.source_diagnostics.diagnostics.len(), 1);
        assert_eq!(
            context.source_diagnostics.diagnostics[0].message,
            "unused value"
        );
    }

    #[test]
    fn compilation_context_can_add_multiple_diagnostics() {
        let mut context = CompilationContext::default();

        context.add_diagnostics([
            SourceDiagnostic::new(
                DiagnosticLevel::Warning,
                "examples/test.ocelot",
                "unused value",
            ),
            SourceDiagnostic::new(
                DiagnosticLevel::Error,
                "examples/test.ocelot",
                "type mismatch",
            ),
        ]);

        assert_eq!(context.source_diagnostics.diagnostics.len(), 2);
        assert!(context.has_errors());
    }
}
