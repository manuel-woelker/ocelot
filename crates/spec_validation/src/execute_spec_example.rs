use crate::capturing_pal::CapturingPal;
use crate::normalize_validation_text::normalize_validation_text;
use crate::observed_outcome::ObservedOutcome;
use crate::render_validation_error::render_validation_error;
use crate::spec_example::SpecExample;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::result::OptionExt;
use ocelot_base::shared_string::SharedString;
use ocelot_engine::engine::Engine;
use ocelot_pal::pal::{Pal, PalHandle};

/// Executes one extracted spec example through the engine pipeline.
pub fn execute_spec_example(
    pal: &CapturingPal,
    execution_root: &std::path::Path,
    example: &SpecExample,
) -> OcelotResult<ObservedOutcome> {
    pal.clear_virtual_files();
    for source_file in &example.source_files {
        let file_path = build_example_file_path(execution_root, &source_file.path);
        pal.write_file(&file_path, source_file.source.as_bytes())?;
    }
    pal.take_printed_output();

    let engine = Engine::new(PalHandle::new(pal.clone()));
    let entry_path = main_example_path(execution_root, example)?;
    let observed = match engine.run_script(&entry_path) {
        Ok(()) => ObservedOutcome::Output(SharedString::from(normalize_validation_text(
            &normalize_example_text(&pal.take_printed_output(), execution_root),
        ))),
        Err(error) => {
            pal.take_printed_output();
            ObservedOutcome::Error(SharedString::from(normalize_validation_text(
                &normalize_example_text(&render_validation_error(&error), execution_root),
            )))
        }
    };

    Ok(observed)
}

fn main_example_path(
    execution_root: &std::path::Path,
    example: &SpecExample,
) -> OcelotResult<FilePath> {
    let main_file = example
        .source_files
        .iter()
        .find(|source_file| source_file.path.as_str() == "main.ocelot")
        .context("spec example is missing `main.ocelot`")?;
    Ok(build_example_file_path(execution_root, &main_file.path))
}

fn build_example_file_path(execution_root: &std::path::Path, relative_path: &FilePath) -> FilePath {
    FilePath::new(execution_root.join(relative_path.as_path()))
}

fn normalize_example_text(text: &str, execution_root: &std::path::Path) -> String {
    let execution_root = execution_root.to_string_lossy();
    let prefix = if execution_root.ends_with(std::path::MAIN_SEPARATOR) {
        execution_root.into_owned()
    } else {
        format!("{}{sep}", execution_root, sep = std::path::MAIN_SEPARATOR)
    };

    text.replace(&prefix, "")
}

#[cfg(test)]
mod tests {
    use super::execute_spec_example;
    use crate::capturing_pal::CapturingPal;
    use crate::expected_outcome::ExpectedOutcome;
    use crate::observed_outcome::ObservedOutcome;
    use crate::spec_example::SpecExample;
    use crate::spec_example_file::SpecExampleFile;
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
                chapter_path: FilePath::from("docs/spec/30.01 Standard library - println.md"),
                name: SharedString::from("writes one line"),
                source_files: vec![SpecExampleFile::new("main.ocelot", "println(\"hello\");")],
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
            pal.read_file_to_string(&FilePath::from("/tmp/spec-validation/main.ocelot"))
                .unwrap(),
            "println(\"hello\");"
        );
    }

    #[test]
    fn executes_multi_file_examples_through_the_engine() {
        let inner_pal = PalMock::new();
        let pal = CapturingPal::new(PalHandle::new(inner_pal));
        let observed = execute_spec_example(
            &pal,
            Path::new("/tmp/spec-validation"),
            &SpecExample {
                chapter_path: FilePath::from("docs/spec/25.01 Modules - File modules.md"),
                name: SharedString::from("calls a sibling module"),
                source_files: vec![
                    SpecExampleFile::new("main.ocelot", "helper::greet();"),
                    SpecExampleFile::new("helper.ocelot", "fun greet() { println(\"hello\"); }"),
                ],
                expected_outcome: ExpectedOutcome::Output(SharedString::from("hello")),
                line_number: 10,
            },
        )
        .unwrap();

        assert_eq!(
            observed,
            ObservedOutcome::Output(SharedString::from("hello"))
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
                chapter_path: FilePath::from("docs/spec/30.01 Standard library - println.md"),
                name: SharedString::from("requires one argument"),
                source_files: vec![SpecExampleFile::new("main.ocelot", "println();")],
                expected_outcome: ExpectedOutcome::Error(SharedString::from("type error")),
                line_number: 18,
            },
        )
        .unwrap();

        assert_eq!(
            observed,
            ObservedOutcome::Error(SharedString::from(
                "error: type error: `println` expects exactly one argument\n  ╭▸ main.ocelot:1:9\n  │\n1 │ println();\n  ╰╴        ━ missing argument\nat main.ocelot:1:9"
            ))
        );
    }

    #[test]
    fn normalizes_unknown_module_resolver_failures_for_comparison() {
        let inner_pal = PalMock::new();
        let pal = CapturingPal::new(PalHandle::new(inner_pal));
        let observed = execute_spec_example(
            &pal,
            Path::new("/tmp/spec-validation"),
            &SpecExample {
                chapter_path: FilePath::from("docs/spec/25.01 Modules - File modules.md"),
                name: SharedString::from("calling an unknown module is a resolver error"),
                source_files: vec![SpecExampleFile::new("main.ocelot", "helper::greet();")],
                expected_outcome: ExpectedOutcome::Error(SharedString::from("unknown module")),
                line_number: 24,
            },
        )
        .unwrap();

        assert_eq!(
            observed,
            ObservedOutcome::Error(SharedString::from(
                "error: unknown module `helper`\n  ╭▸ main.ocelot:1:1\n  │\n1 │ helper::greet();\n  ╰╴━━━━━━━━━━━━━ unknown module\nat main.ocelot:1:1"
            ))
        );
    }
}
