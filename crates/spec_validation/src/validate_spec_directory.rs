use crate::capturing_pal::CapturingPal;
use crate::execute_spec_example::execute_spec_example;
use crate::expected_outcome::ExpectedOutcome;
use crate::load_spec_chapters::load_spec_chapters;
use crate::observed_outcome::ObservedOutcome;
use crate::validation_failure::ValidationFailure;
use crate::validation_failure_kind::ValidationFailureKind;
use crate::validation_report::ValidationReport;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;

/// Validates a spec directory end to end.
pub fn validate_spec_directory(
    pal: &CapturingPal,
    spec_root: &FilePath,
    execution_root: &std::path::Path,
) -> OcelotResult<ValidationReport> {
    let chapters = load_spec_chapters(pal, spec_root)?;
    let scanned_chapter_count = chapters.len();
    let example_count = chapters.iter().map(|chapter| chapter.examples.len()).sum();
    let mut passed_example_count = 0usize;
    let mut failures = chapters
        .iter()
        .flat_map(|chapter| chapter.malformed_failures.iter().cloned())
        .collect::<Vec<_>>();

    for chapter in &chapters {
        for example in &chapter.examples {
            let observed_outcome = execute_spec_example(pal, execution_root, example)?;

            match (&example.expected_outcome, observed_outcome) {
                (ExpectedOutcome::Output(expected), ObservedOutcome::Output(actual))
                    if actual.as_str() == expected.as_str() =>
                {
                    passed_example_count += 1;
                }
                (ExpectedOutcome::Error(expected), ObservedOutcome::Error(actual))
                    if actual.as_str() == expected.as_str() =>
                {
                    passed_example_count += 1;
                }
                (ExpectedOutcome::Output(expected), ObservedOutcome::Output(actual)) => {
                    failures.push(ValidationFailure {
                        chapter_path: example.chapter_path.clone(),
                        example_name: example.name.clone(),
                        kind: ValidationFailureKind::OutputMismatch,
                        message: SharedString::from("output mismatch"),
                        expected_output: Some(expected.clone()),
                        actual_output: Some(actual),
                        line_number: example.line_number,
                    });
                }
                (ExpectedOutcome::Error(expected), ObservedOutcome::Error(actual)) => {
                    failures.push(ValidationFailure {
                        chapter_path: example.chapter_path.clone(),
                        example_name: example.name.clone(),
                        kind: ValidationFailureKind::ErrorMismatch,
                        message: SharedString::from("error mismatch"),
                        expected_output: Some(expected.clone()),
                        actual_output: Some(actual),
                        line_number: example.line_number,
                    });
                }
                (ExpectedOutcome::Error(expected), ObservedOutcome::Output(actual)) => {
                    failures.push(ValidationFailure {
                        chapter_path: example.chapter_path.clone(),
                        example_name: example.name.clone(),
                        kind: ValidationFailureKind::ExpectedErrorButSucceeded,
                        message: SharedString::from("example succeeded but an error was expected"),
                        expected_output: Some(expected.clone()),
                        actual_output: Some(actual),
                        line_number: example.line_number,
                    });
                }
                (ExpectedOutcome::Output(expected), ObservedOutcome::Error(actual)) => {
                    failures.push(ValidationFailure {
                        chapter_path: example.chapter_path.clone(),
                        example_name: example.name.clone(),
                        kind: ValidationFailureKind::ExpectedOutputButFailed,
                        message: SharedString::from("example failed but output was expected"),
                        expected_output: Some(expected.clone()),
                        actual_output: Some(actual),
                        line_number: example.line_number,
                    });
                }
            }
        }
    }

    Ok(ValidationReport {
        scanned_chapter_count,
        example_count,
        passed_example_count,
        failures,
    })
}

#[cfg(test)]
mod tests {
    use super::validate_spec_directory;
    use crate::capturing_pal::CapturingPal;
    use crate::validation_failure_kind::ValidationFailureKind;
    use ocelot_base::file_path::FilePath;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;
    use std::path::Path;

    #[test]
    fn validates_a_synthetic_spec_directory_without_depending_on_repo_docs() {
        let inner_pal = PalMock::new();
        inner_pal.set_file(
            "docs/spec/01.01 Lexical structure - Comments.md",
            r#"
## Example: output example

main.ocelot-script:

```ocelot
println("hello");
```

### Output

```text
hello
```
"#,
        );
        inner_pal.set_file(
            "docs/spec/02.01 Expressions - Function calls.md",
            r#"
## Example: error example

main.ocelot-script:

```ocelot
helper::greet();
```

### Error

```text
error: unknown module `helper`
  ╭▸ main.ocelot-script:1:1
  │
1 │ helper::greet();
  ╰╴━━━━━━━━━━━━━ unknown module
at main.ocelot-script:1:1
```
"#,
        );
        inner_pal.set_file(
            "docs/spec/03.01 Broken chapter.md",
            r#"
## Example: missing expectation

main.ocelot-script:

```ocelot
println("oops");
```
"#,
        );
        inner_pal.set_file(
            "docs/spec/README.md",
            "# ignored because it is not a numbered chapter\n",
        );
        let pal = CapturingPal::new(PalHandle::new(inner_pal));

        let report = validate_spec_directory(
            &pal,
            &FilePath::from("docs/spec"),
            Path::new("/tmp/spec-validation"),
        )
        .unwrap();

        assert_eq!(report.scanned_chapter_count, 3);
        assert_eq!(report.example_count, 2);
        assert_eq!(report.passed_example_count, 2);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(
            report.failures[0].kind,
            ValidationFailureKind::MalformedExample
        );
        assert_eq!(
            report.failures[0].message.as_str(),
            "example is missing its `### Output` or `### Error` `text` block"
        );
    }

    #[test]
    fn reports_output_mismatches() {
        let inner_pal = PalMock::new();
        inner_pal.set_file(
            "docs/spec/28.01 Runtime behavior - Scripts.md",
            r#"
## Example: mismatch

main.ocelot-script:

```ocelot
println("hello");
```

### Output

```text
goodbye
```
"#,
        );
        let pal = CapturingPal::new(PalHandle::new(inner_pal));

        let report = validate_spec_directory(
            &pal,
            &FilePath::from("docs/spec"),
            Path::new("/tmp/spec-validation"),
        )
        .unwrap();

        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].message.as_str(), "output mismatch");
        assert_eq!(
            report.failures[0]
                .actual_output
                .as_ref()
                .map(|text| text.as_str()),
            Some("hello")
        );
    }

    #[test]
    fn reports_error_mismatches() {
        let inner_pal = PalMock::new();
        inner_pal.set_file(
            "docs/spec/30.01 Standard library - println.md",
            r#"
## Example: requires one argument

main.ocelot-script:

```ocelot
println();
```

### Error

```text
different error
```
"#,
        );
        let pal = CapturingPal::new(PalHandle::new(inner_pal));

        let report = validate_spec_directory(
            &pal,
            &FilePath::from("docs/spec"),
            Path::new("/tmp/spec-validation"),
        )
        .unwrap();

        assert_eq!(report.failures.len(), 1);
        assert_eq!(
            report.failures[0].kind,
            ValidationFailureKind::ErrorMismatch
        );
        assert_eq!(report.failures[0].message.as_str(), "error mismatch");
    }

    #[test]
    fn reports_success_when_error_was_expected() {
        let inner_pal = PalMock::new();
        inner_pal.set_file(
            "docs/spec/30.01 Standard library - println.md",
            r#"
## Example: should fail

main.ocelot-script:

```ocelot
println("hello");
```

### Error

```text
some error
```
"#,
        );
        let pal = CapturingPal::new(PalHandle::new(inner_pal));

        let report = validate_spec_directory(
            &pal,
            &FilePath::from("docs/spec"),
            Path::new("/tmp/spec-validation"),
        )
        .unwrap();

        assert_eq!(
            report.failures[0].kind,
            ValidationFailureKind::ExpectedErrorButSucceeded
        );
    }

    #[test]
    fn reports_failure_when_output_was_expected() {
        let inner_pal = PalMock::new();
        inner_pal.set_file(
            "docs/spec/30.01 Standard library - println.md",
            r#"
## Example: should print

main.ocelot-script:

```ocelot
println();
```

### Output

```text
hello
```
"#,
        );
        let pal = CapturingPal::new(PalHandle::new(inner_pal));

        let report = validate_spec_directory(
            &pal,
            &FilePath::from("docs/spec"),
            Path::new("/tmp/spec-validation"),
        )
        .unwrap();

        assert_eq!(
            report.failures[0].kind,
            ValidationFailureKind::ExpectedOutputButFailed
        );
    }
}
