use crate::capturing_pal::CapturingPal;
use crate::normalize_validation_text::normalize_validation_text;
use crate::observed_outcome::ObservedOutcome;
use crate::render_validation_error::render_validation_error;
use crate::spec_example::SpecExample;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_engine::engine::Engine;
use ocelot_pal::pal::{Pal, PalHandle};

/// Executes one extracted spec example through the engine pipeline.
pub fn execute_spec_example(
    pal: &CapturingPal,
    execution_root: &std::path::Path,
    example: &SpecExample,
) -> OcelotResult<ObservedOutcome> {
    let example_path = build_example_path(execution_root, example);
    pal.write_file(&example_path, example.source.as_bytes())?;
    pal.take_printed_output();

    let engine = Engine::new(PalHandle::new(pal.clone()));
    let observed = match engine.run_script(&example_path) {
        Ok(()) => ObservedOutcome::Output(SharedString::from(normalize_validation_text(
            &pal.take_printed_output(),
        ))),
        Err(error) => {
            pal.take_printed_output();
            ObservedOutcome::Error(SharedString::from(normalize_validation_text(
                &render_validation_error(&error),
            )))
        }
    };

    Ok(observed)
}

fn build_example_path(execution_root: &std::path::Path, example: &SpecExample) -> FilePath {
    let _ = execution_root;
    let _ = example;
    FilePath::from("spec-test.ocelot")
}

#[cfg(test)]
mod tests {
    use super::execute_spec_example;
    use crate::capturing_pal::CapturingPal;
    use crate::expected_outcome::ExpectedOutcome;
    use crate::observed_outcome::ObservedOutcome;
    use crate::spec_example::SpecExample;
    use ocelot_base::file_path::FilePath;
    use ocelot_base::shared_string::SharedString;
    use ocelot_pal::pal::{Pal, PalHandle};
    use ocelot_pal::pal_mock::PalMock;
    use std::path::Path;

    #[test]
    fn executes_examples_through_the_engine() {
        let inner_pal = PalMock::new();
        let pal = CapturingPal::new(PalHandle::new(inner_pal.clone()));
        let observed = execute_spec_example(
            &pal,
            Path::new("/tmp/spec-validation"),
            &SpecExample {
                chapter_path: FilePath::from("docs/spec/09.01 Standard library - println.md"),
                name: SharedString::from("writes one line"),
                source: SharedString::from("println(\"hello\");"),
                expected_outcome: ExpectedOutcome::Output(SharedString::from("hello")),
                line_number: 10,
            },
        )
        .unwrap();

        assert_eq!(
            observed,
            ObservedOutcome::Output(SharedString::from("hello"))
        );
        assert_eq!(
            pal.read_file_to_string(&FilePath::from("spec-test.ocelot"))
                .unwrap(),
            "println(\"hello\");"
        );
    }

    #[test]
    fn normalizes_engine_failures_for_comparison() {
        let inner_pal = PalMock::new();
        let pal = CapturingPal::new(PalHandle::new(inner_pal));
        let observed = execute_spec_example(
            &pal,
            Path::new("/tmp/spec-validation"),
            &SpecExample {
                chapter_path: FilePath::from("docs/spec/09.01 Standard library - println.md"),
                name: SharedString::from("requires one argument"),
                source: SharedString::from("println();"),
                expected_outcome: ExpectedOutcome::Error(SharedString::from("type error")),
                line_number: 18,
            },
        )
        .unwrap();

        assert_eq!(
            observed,
            ObservedOutcome::Error(SharedString::from(
                "error: type error: `println` expects exactly one argument\n  ╭▸ spec-test.ocelot:1:9\n  │\n1 │ println();\n  ╰╴        ━ missing argument\nat spec-test.ocelot:1"
            ))
        );
    }
}
