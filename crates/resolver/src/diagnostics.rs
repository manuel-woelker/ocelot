use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;

pub(crate) fn source_diagnostic_for_span(
    source_file: &SourceFile,
    message: impl Into<SharedString>,
    span: Span,
    annotation: impl Into<SharedString>,
) -> SourceDiagnostic {
    let message = message.into();
    SourceDiagnostic::new(DiagnosticLevel::Error, &source_file.path, message)
        .with_excerpt(source_excerpt_for_span(source_file, span, annotation))
}

pub(crate) fn source_excerpt_for_span(
    source_file: &SourceFile,
    span: Span,
    annotation: impl Into<SharedString>,
) -> SourceExcerpt {
    let annotation = annotation.into();
    let (line_number, line_start, line_end) = line_bounds(source_file.source(), span.start());
    let source_line = &source_file.source()[line_start..line_end];
    let relative_start = span.start().saturating_sub(line_start);
    let relative_end = span.end().saturating_sub(line_start);

    SourceExcerpt::new(&source_file.path, line_number, source_line).with_annotation(
        SourceAnnotation::new(Span::new(relative_start, relative_end), annotation),
    )
}

fn line_bounds(source: &str, index: usize) -> (usize, usize, usize) {
    let line_start = source[..index].rfind('\n').map_or(0, |offset| offset + 1);
    let line_end = source[index..]
        .find('\n')
        .map_or(source.len(), |offset| index + offset);
    let line_number = source[..line_start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;

    (line_number, line_start, line_end)
}
