use crate::validation_failure_kind::ValidationFailureKind;
use crate::validation_report::ValidationReport;
use std::fmt::Write as _;

/// Renders a validation report for human-readable output.
pub fn render_validation_report(report: &ValidationReport) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        &mut rendered,
        "Validated {} chapters and {} examples: {} passed, {} failed.",
        report.scanned_chapter_count,
        report.example_count,
        report.passed_example_count,
        report.failures.len()
    );

    for failure in &report.failures {
        let kind = match failure.kind {
            ValidationFailureKind::MalformedExample => "malformed example",
            ValidationFailureKind::OutputMismatch => "output mismatch",
            ValidationFailureKind::ErrorMismatch => "error mismatch",
            ValidationFailureKind::ExpectedErrorButSucceeded => {
                "expected error but execution succeeded"
            }
            ValidationFailureKind::ExpectedOutputButFailed => {
                "expected output but execution failed"
            }
        };
        let _ = writeln!(
            &mut rendered,
            "{}:{}: {} in `{}`",
            failure.chapter_path, failure.line_number, kind, failure.example_name
        );
        let _ = writeln!(&mut rendered, "  {}", failure.message);
        if let Some(expected_output) = &failure.expected_output {
            let _ = writeln!(
                &mut rendered,
                "  expected:\n{}",
                indent_block(expected_output)
            );
        }
        if let Some(actual_output) = &failure.actual_output {
            let _ = writeln!(&mut rendered, "  actual:\n{}", indent_block(actual_output));
        }
    }

    rendered
}

fn indent_block(text: &str) -> String {
    if text.is_empty() {
        return String::from("    <empty>");
    }

    text.lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::render_validation_report;
    use crate::validation_failure::ValidationFailure;
    use crate::validation_failure_kind::ValidationFailureKind;
    use crate::validation_report::ValidationReport;
    use expect_test::expect;
    use ocelot_base::file_path::FilePath;
    use ocelot_base::shared_string::SharedString;

    #[test]
    fn renders_summary_and_failure_details() {
        let rendered = render_validation_report(&ValidationReport {
            scanned_chapter_count: 2,
            example_count: 5,
            passed_example_count: 4,
            failures: vec![ValidationFailure {
                chapter_path: FilePath::from("docs/spec/30.01 Standard library - println.md"),
                example_name: SharedString::from("requires one argument"),
                kind: ValidationFailureKind::ErrorMismatch,
                message: SharedString::from("error mismatch"),
                expected_output: Some(SharedString::from("expected")),
                actual_output: Some(SharedString::from("actual")),
                line_number: 30,
            }],
        });

        expect![[r#"
            Validated 2 chapters and 5 examples: 4 passed, 1 failed.
            docs/spec/30.01 Standard library - println.md:30: error mismatch in `requires one argument`
              error mismatch
              expected:
                expected
              actual:
                actual
        "#]]
        .assert_eq(&rendered);
    }

    #[test]
    fn renders_wrong_execution_outcome_failures() {
        let rendered = render_validation_report(&ValidationReport {
            scanned_chapter_count: 1,
            example_count: 1,
            passed_example_count: 0,
            failures: vec![ValidationFailure {
                chapter_path: FilePath::from("docs/spec/30.01 Standard library - println.md"),
                example_name: SharedString::from("requires one argument"),
                kind: ValidationFailureKind::ExpectedErrorButSucceeded,
                message: SharedString::from("example succeeded but an error was expected"),
                expected_output: Some(SharedString::from("type error")),
                actual_output: Some(SharedString::from("hello")),
                line_number: 30,
            }],
        });

        expect![[r#"
            Validated 1 chapters and 1 examples: 0 passed, 1 failed.
            docs/spec/30.01 Standard library - println.md:30: expected error but execution succeeded in `requires one argument`
              example succeeded but an error was expected
              expected:
                type error
              actual:
                hello
        "#]]
        .assert_eq(&rendered);
    }
}
