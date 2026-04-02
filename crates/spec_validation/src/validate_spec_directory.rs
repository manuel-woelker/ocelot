use crate::capturing_pal::CapturingPal;
use crate::execute_spec_example::execute_spec_example;
use crate::load_spec_chapters::load_spec_chapters;
use crate::validation_failure::ValidationFailure;
use crate::validation_failure_kind::ValidationFailureKind;
use crate::validation_report::ValidationReport;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;

/// How is a spec directory validated end to end?
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
            let observed_output = execute_spec_example(pal, execution_root, example)?;
            if observed_output == example.expected_output.as_str() {
                passed_example_count += 1;
                continue;
            }

            failures.push(ValidationFailure {
                chapter_path: example.chapter_path.clone(),
                example_name: example.name.clone(),
                kind: ValidationFailureKind::OutputMismatch,
                message: SharedString::from("output mismatch"),
                expected_output: Some(example.expected_output.clone()),
                actual_output: Some(SharedString::from(observed_output)),
                line_number: example.line_number,
            });
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
    use expect_test::expect;
    use ocelot_base::file_path::FilePath;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;
    use std::path::Path;

    #[test]
    fn validates_the_real_spec_directory() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let inner_pal = PalMock::new();
        inner_pal.set_file(
            "docs/spec/08.01 Runtime behavior - Scripts.md",
            std::fs::read_to_string(
                repo_root.join("docs/spec/08.01 Runtime behavior - Scripts.md"),
            )
            .unwrap(),
        );
        inner_pal.set_file(
            "docs/spec/09.01 Standard library - println.md",
            std::fs::read_to_string(
                repo_root.join("docs/spec/09.01 Standard library - println.md"),
            )
            .unwrap(),
        );
        let pal = CapturingPal::new(PalHandle::new(inner_pal));

        let report = validate_spec_directory(
            &pal,
            &FilePath::from("docs/spec"),
            Path::new("/tmp/spec-validation"),
        )
        .unwrap();

        expect![[r#"
            2
            5
            5
            0
        "#]]
        .assert_eq(&format!(
            "{}\n{}\n{}\n{}\n",
            report.scanned_chapter_count,
            report.example_count,
            report.passed_example_count,
            report.failures.len()
        ));
    }

    #[test]
    fn reports_output_mismatches() {
        let inner_pal = PalMock::new();
        inner_pal.set_file(
            "docs/spec/08.01 Runtime behavior - Scripts.md",
            r#"
## Example: mismatch

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
}
