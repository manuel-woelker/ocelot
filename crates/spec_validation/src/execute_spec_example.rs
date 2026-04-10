use crate::capturing_pal::CapturingPal;
use crate::normalize_validation_text::normalize_validation_text;
use crate::observed_outcome::ObservedOutcome;
use crate::render_validation_error::render_validation_error;
use crate::spec_example::SpecExample;
use crate::spec_example_embedded_step_kind::SpecExampleEmbeddedStepKind;
use crate::spec_example_file::SpecExampleFile;
use ocelot_base::error::OcelotError;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_base::source_file::SourceFile;
use ocelot_engine::embedded_engine::EmbeddedEngine;
use ocelot_engine::engine::Engine;
use ocelot_formatter::format_compilation_unit::format_compilation_unit;
use ocelot_pal::pal::{Pal, PalHandle};

/// Executes one extracted spec example through the engine pipeline.
pub fn execute_spec_example(
    pal: &CapturingPal,
    execution_root: &std::path::Path,
    example: &SpecExample,
) -> OcelotResult<ObservedOutcome> {
    validate_example_formatting(example)?;
    pal.clear_virtual_files();
    for source_file in &example.source_files {
        let file_path = build_example_file_path(execution_root, &source_file.path);
        pal.write_file(&file_path, source_file.source.as_bytes())?;
    }
    pal.take_printed_output();

    if !example.embedded_steps.is_empty() {
        return execute_embedded_example(pal, execution_root, example);
    }

    let engine = Engine::new(PalHandle::new(pal.clone()));
    let entry_path = main_example_path(execution_root, example)?;
    let observed = match engine.run_file(&entry_path) {
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

fn execute_embedded_example(
    pal: &CapturingPal,
    execution_root: &std::path::Path,
    example: &SpecExample,
) -> OcelotResult<ObservedOutcome> {
    let mut engine =
        EmbeddedEngine::new(PalHandle::new(pal.clone()), FilePath::new(execution_root));
    let mut last_eval_result = None;

    for step in &example.embedded_steps {
        let result = match step.kind {
            SpecExampleEmbeddedStepKind::Execute => {
                engine.execute(step.source.as_str()).map(|_| None)
            }
            SpecExampleEmbeddedStepKind::Eval => engine.eval(step.source.as_str()).map(Some),
        };

        match result {
            Ok(eval_result) => {
                if let Some(eval_result) = eval_result {
                    last_eval_result = Some(eval_result);
                }
            }
            Err(error) => {
                pal.take_printed_output();
                return Ok(ObservedOutcome::Error(SharedString::from(
                    normalize_validation_text(&normalize_example_text(
                        &render_validation_error(&error),
                        execution_root,
                    )),
                )));
            }
        }
    }

    let eval_result = last_eval_result
        .expect("embedded examples should be validated to include at least one eval step");

    Ok(ObservedOutcome::EvalResult(SharedString::from(
        normalize_validation_text(eval_result.render_for_eval_result().as_str()),
    )))
}

fn validate_example_formatting(example: &SpecExample) -> OcelotResult<()> {
    let mut misformatted_files = Vec::new();

    for source_file in &example.source_files {
        if !is_ocelot_source_file(source_file) {
            continue;
        }

        validate_formatted_compilation_unit(
            source_file.path.as_str(),
            source_file.source.as_str(),
            &mut misformatted_files,
        );
    }

    for step in &example.embedded_steps {
        if step.kind != SpecExampleEmbeddedStepKind::Execute {
            continue;
        }

        validate_formatted_compilation_unit(
            "execute:",
            step.source.as_str(),
            &mut misformatted_files,
        );
    }

    if misformatted_files.is_empty() {
        return Ok(());
    }

    let mut message = format!(
        "spec example source files must already match formatter output for `{}`",
        example.name
    );

    for (label, formatted) in misformatted_files {
        message.push_str("\n\n");
        message.push_str(&label);
        message.push_str(" expected format:\n```ocelot\n");
        message.push_str(&formatted);
        message.push_str("\n```");
    }

    Err(OcelotError::message(message))
}

fn validate_formatted_compilation_unit(
    label: &str,
    source: &str,
    misformatted_files: &mut Vec<(String, String)>,
) {
    let source_file_for_parser = SourceFile::new(label, source);
    let mut source_diagnostics = SourceDiagnostics::default();
    let parsed = ocelot_parser::parse_compilation_unit::parse_compilation_unit(
        &source_file_for_parser,
        &mut source_diagnostics,
    );

    if parsed.is_err() || source_diagnostics.has_errors() {
        return;
    }

    let formatted = format_compilation_unit(&parsed.expect("successful parse must exist"));
    if formatted != source {
        misformatted_files.push((label.to_owned(), formatted));
    }
}

fn main_example_path(
    execution_root: &std::path::Path,
    example: &SpecExample,
) -> OcelotResult<FilePath> {
    let script_file = example
        .source_files
        .iter()
        .find(|source_file| source_file.path.as_str() == "main.ocelot-script");
    let module_file = example
        .source_files
        .iter()
        .find(|source_file| source_file.path.as_str() == "main.ocelot");
    let main_file = match (script_file, module_file) {
        (Some(_), Some(_)) => {
            ocelot_base::bail!(
                "spec example must not declare both `main.ocelot` and `main.ocelot-script`"
            )
        }
        (Some(file), None) | (None, Some(file)) => file,
        (None, None) => {
            ocelot_base::bail!("spec example must declare `main.ocelot` or `main.ocelot-script`")
        }
    };
    Ok(build_example_file_path(execution_root, &main_file.path))
}

fn build_example_file_path(execution_root: &std::path::Path, relative_path: &FilePath) -> FilePath {
    FilePath::new(execution_root.join(relative_path.as_path()))
}

fn is_ocelot_source_file(source_file: &SpecExampleFile) -> bool {
    source_file.path.as_str().ends_with(".ocelot")
        || source_file.path.as_str().ends_with(".ocelot-script")
}

fn normalize_example_text(text: &str, execution_root: &std::path::Path) -> String {
    let execution_root = execution_root.to_string_lossy();
    let prefix = if execution_root.ends_with(std::path::MAIN_SEPARATOR) {
        execution_root.into_owned()
    } else {
        format!("{}{sep}", execution_root, sep = std::path::MAIN_SEPARATOR)
    };

    text.replace(&prefix, "")
        .replace("__embedded_main.ocelot-script", "<embedded>")
}

#[cfg(test)]
mod tests {
    use super::execute_spec_example;
    use crate::capturing_pal::CapturingPal;
    use crate::expected_outcome::ExpectedOutcome;
    use crate::observed_outcome::ObservedOutcome;
    use crate::spec_example::SpecExample;
    use crate::spec_example_embedded_step::SpecExampleEmbeddedStep;
    use crate::spec_example_embedded_step_kind::SpecExampleEmbeddedStepKind;
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
                source_files: vec![SpecExampleFile::new(
                    "main.ocelot-script",
                    "println(\"hello\");",
                )],
                embedded_steps: Vec::new(),
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
            pal.read_file_to_string(&FilePath::from("/tmp/spec-validation/main.ocelot-script"))
                .unwrap(),
            "println(\"hello\");"
        );
    }

    #[test]
    fn executes_embedded_eval_examples_through_the_embedded_engine() {
        let inner_pal = PalMock::new();
        let pal = CapturingPal::new(PalHandle::new(inner_pal));
        let observed = execute_spec_example(
            &pal,
            Path::new("/tmp/spec-validation"),
            &SpecExample {
                chapter_path: FilePath::from("docs/spec/28.03 Runtime behavior - Embedded eval.md"),
                name: SharedString::from("embedded eval keeps prior definitions"),
                source_files: vec![SpecExampleFile::new(
                    "helper.ocelot",
                    "fun greet() {\n    println(\"hello\");\n}",
                )],
                embedded_steps: vec![
                    SpecExampleEmbeddedStep::new(
                        SpecExampleEmbeddedStepKind::Execute,
                        "fun local() {\n    helper::greet();\n}",
                    ),
                    SpecExampleEmbeddedStep::new(SpecExampleEmbeddedStepKind::Eval, "local()"),
                ],
                expected_outcome: ExpectedOutcome::EvalResult(SharedString::from("()")),
                line_number: 10,
            },
        )
        .unwrap();

        assert_eq!(
            observed,
            ObservedOutcome::EvalResult(SharedString::from("()"))
        );
        assert_eq!(pal.take_printed_output(), "hello\n");
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
                source_files: vec![SpecExampleFile::new("main.ocelot-script", "println();")],
                embedded_steps: Vec::new(),
                expected_outcome: ExpectedOutcome::Error(SharedString::from("type error")),
                line_number: 18,
            },
        )
        .unwrap();

        assert_eq!(
            observed,
            ObservedOutcome::Error(SharedString::from(
                "error: type error: `println` expects exactly one argument\n  ╭▸ main.ocelot-script:1:1\n  │\n1 │ println();\n  ╰╴━━━━━━━ missing argument\nat main.ocelot-script:1:1"
            ))
        );
    }

    #[test]
    fn rejects_misformatted_example_files_before_execution() {
        let inner_pal = PalMock::new();
        let pal = CapturingPal::new(PalHandle::new(inner_pal));
        let error = execute_spec_example(
            &pal,
            Path::new("/tmp/spec-validation"),
            &SpecExample {
                chapter_path: FilePath::from(
                    "docs/spec/15.02 Declarations - Function definitions.md",
                ),
                name: SharedString::from("misformatted example"),
                source_files: vec![SpecExampleFile::new(
                    "main.ocelot-script",
                    "println( \"hello\" );",
                )],
                embedded_steps: Vec::new(),
                expected_outcome: ExpectedOutcome::Output(SharedString::from("hello")),
                line_number: 10,
            },
        )
        .unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("spec example source files must already match formatter output")
        );
        assert!(
            error
                .to_test_string()
                .contains("main.ocelot-script expected format:")
        );
        assert!(error.to_test_string().contains("println(\"hello\");"));
    }

    #[test]
    fn rejects_misformatted_execute_steps_before_execution() {
        let inner_pal = PalMock::new();
        let pal = CapturingPal::new(PalHandle::new(inner_pal));
        let error = execute_spec_example(
            &pal,
            Path::new("/tmp/spec-validation"),
            &SpecExample {
                chapter_path: FilePath::from("docs/spec/28.03 Runtime behavior - Embedded eval.md"),
                name: SharedString::from("misformatted embedded step"),
                source_files: Vec::new(),
                embedded_steps: vec![
                    SpecExampleEmbeddedStep::new(
                        SpecExampleEmbeddedStepKind::Execute,
                        "fun greet(){ println(\"hello\"); }",
                    ),
                    SpecExampleEmbeddedStep::new(SpecExampleEmbeddedStepKind::Eval, "greet()"),
                ],
                expected_outcome: ExpectedOutcome::EvalResult(SharedString::from("()")),
                line_number: 10,
            },
        )
        .unwrap_err();

        assert!(error.to_test_string().contains("execute: expected format:"));
    }
}
