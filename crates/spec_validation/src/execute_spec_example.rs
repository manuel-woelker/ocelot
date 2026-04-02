use crate::capturing_pal::CapturingPal;
use crate::normalize_validation_text::normalize_validation_text;
use crate::render_validation_error::render_validation_error;
use crate::spec_example::SpecExample;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_engine::engine::Engine;
use ocelot_pal::pal::{Pal, PalHandle};

/// Executes one extracted spec example through the engine pipeline.
pub fn execute_spec_example(
    pal: &CapturingPal,
    execution_root: &std::path::Path,
    example: &SpecExample,
) -> OcelotResult<String> {
    let example_path = build_example_path(execution_root, example);
    let example_directory = example_path
        .parent()
        .expect("generated example path should always have a parent");

    pal.create_directory_all(&example_directory)?;
    pal.write_file(&example_path, example.source.as_bytes())?;
    pal.take_printed_output();

    let engine = Engine::new(PalHandle::new(pal.clone()));
    let observed = match engine.run_script(&example_path) {
        Ok(()) => pal.take_printed_output(),
        Err(error) => {
            pal.take_printed_output();
            render_validation_error(&error)
        }
    };

    Ok(normalize_validation_text(&observed))
}

fn build_example_path(execution_root: &std::path::Path, example: &SpecExample) -> FilePath {
    let chapter_stem = example
        .chapter_path
        .file_stem()
        .unwrap_or("spec-chapter")
        .replace(['/', '\\'], "_");
    let file_name = format!("{chapter_stem}-line-{}.ocelot", example.line_number);
    FilePath::new(execution_root.join(file_name))
}

#[cfg(test)]
mod tests {
    use super::execute_spec_example;
    use crate::capturing_pal::CapturingPal;
    use crate::spec_example::SpecExample;
    use ocelot_base::file_path::FilePath;
    use ocelot_base::shared_string::SharedString;
    use ocelot_pal::pal::PalHandle;
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
                expected_output: SharedString::from("hello"),
                line_number: 10,
            },
        )
        .unwrap();

        assert_eq!(observed, "hello");
        assert_eq!(
            inner_pal
                .read_file_string(
                    "/tmp/spec-validation/09.01 Standard library - println-line-10.ocelot"
                )
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
                expected_output: SharedString::from("type error"),
                line_number: 18,
            },
        )
        .unwrap();

        assert_eq!(
            observed,
            "type error: `println` expects exactly one argument"
        );
    }
}
