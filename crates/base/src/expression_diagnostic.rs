use crate::diagnostic_level::DiagnosticLevel;
use crate::line_bounds::LineBounds;
use crate::shared_string::SharedString;
use crate::source_annotation::SourceAnnotation;
use crate::source_diagnostic::SourceDiagnostic;
use crate::source_excerpt::SourceExcerpt;
use crate::source_file::SourceFile;
use crate::span::Span;

/// Builds a source error, clipping multiline expressions to their first line.
pub fn expression_diagnostic(
    source: &SourceFile,
    message: impl Into<SharedString>,
    span: Span,
    annotation: impl Into<SharedString>,
) -> SourceDiagnostic {
    let bounds = LineBounds::new(source.source(), span.start());
    let line = &source.source()[bounds.line_start..bounds.line_end];
    SourceDiagnostic::new(DiagnosticLevel::Error, &source.path, message).with_excerpt(
        SourceExcerpt::new(&source.path, bounds.line_number, line).with_annotation(
            SourceAnnotation::new(
                Span::new(
                    span.start() - bounds.line_start,
                    span.end().min(bounds.line_end) - bounds.line_start,
                ),
                annotation,
            ),
        ),
    )
}
