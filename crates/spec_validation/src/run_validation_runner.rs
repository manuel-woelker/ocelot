use crate::capturing_pal::CapturingPal;
use crate::render_validation_report::render_validation_report;
use crate::validate_spec_directory::validate_spec_directory;
use ocelot_base::bail;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;

/// Runs the spec validation runner.
pub fn run_validation_runner(
    pal: &CapturingPal,
    spec_root: &FilePath,
    execution_root: &std::path::Path,
) -> OcelotResult<()> {
    let report = validate_spec_directory(pal, spec_root, execution_root)?;
    println!("{}", render_validation_report(&report));

    if report.is_success() {
        return Ok(());
    }

    bail!("spec validation found {} failure(s)", report.failures.len())
}

#[cfg(test)]
mod tests {
    use super::run_validation_runner;
    use crate::capturing_pal::CapturingPal;
    use ocelot_base::file_path::FilePath;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;
    use std::path::Path;

    #[test]
    fn returns_an_error_when_validation_fails() {
        let inner_pal = PalMock::new();
        inner_pal.set_file(
            "docs/spec/28.01 Runtime behavior - Scripts.md",
            r#"
## Example: mismatch

main.ocelot:

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

        let error = run_validation_runner(
            &pal,
            &FilePath::from("docs/spec"),
            Path::new("/tmp/spec-validation"),
        )
        .unwrap_err();

        assert_eq!(
            error.kind().to_string(),
            "spec validation found 1 failure(s)"
        );
    }
}
