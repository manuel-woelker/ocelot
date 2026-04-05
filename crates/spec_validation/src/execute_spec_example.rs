use crate::capturing_pal::CapturingPal;
use crate::normalize_validation_text::normalize_validation_text;
use crate::observed_outcome::ObservedOutcome;
use crate::render_validation_error::render_validation_error;
use crate::spec_example::SpecExample;
use crate::spec_example_file::SpecExampleFile;
use ocelot_base::error::OcelotError;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_base::source_file::SourceFile;
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

fn validate_example_formatting(example: &SpecExample) -> OcelotResult<()> {
    let mut misformatted_files = Vec::new();

    for source_file in &example.source_files {
        if !is_ocelot_source_file(source_file) {
            continue;
        }

        let source_file_for_parser =
            SourceFile::new(&source_file.path, source_file.source.as_str());
        let mut source_diagnostics = SourceDiagnostics::default();
        let parsed = ocelot_parser::parse_compilation_unit::parse_compilation_unit(
            &source_file_for_parser,
            &mut source_diagnostics,
        );

        if parsed.is_err() || source_diagnostics.has_errors() {
            continue;
        }

        let formatted = format_compilation_unit(&parsed.expect("successful parse must exist"));
        if formatted != source_file.source.as_str() {
            misformatted_files.push((source_file, formatted));
        }
    }

    if misformatted_files.is_empty() {
        return Ok(());
    }

    let mut message = format!(
        "spec example source files must already match formatter output for `{}`",
        example.name
    );

    for (source_file, formatted) in misformatted_files {
        message.push_str("\n\n");
        message.push_str(source_file.path.as_str());
        message.push_str(" expected format:\n```ocelot\n");
        message.push_str(&formatted);
        message.push_str("\n```");
    }

    Err(OcelotError::message(message))
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
                source_files: vec![SpecExampleFile::new(
                    "main.ocelot-script",
                    "println(\"hello\");",
                )],
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
            pal.read_file_to_string(&FilePath::from("/tmp/spec-validation/main.ocelot-script",))
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
                    SpecExampleFile::new("main.ocelot-script", "helper::greet();"),
                    SpecExampleFile::new(
                        "helper.ocelot",
                        "fun greet() {\n    println(\"hello\");\n}",
                    ),
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
    fn executes_import_examples_through_the_engine() {
        let inner_pal = PalMock::new();
        let pal = CapturingPal::new(PalHandle::new(inner_pal));
        let observed = execute_spec_example(
            &pal,
            Path::new("/tmp/spec-validation"),
            &SpecExample {
                chapter_path: FilePath::from("docs/spec/25.02 Modules - Imports.md"),
                name: SharedString::from("imports one sibling function"),
                source_files: vec![
                    SpecExampleFile::new("main.ocelot-script", "use helper::greet;\ngreet();"),
                    SpecExampleFile::new(
                        "helper.ocelot",
                        "fun greet() {\n    println(\"hello\");\n}",
                    ),
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
                source_files: vec![SpecExampleFile::new("main.ocelot-script", "println();")],
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
    fn normalizes_unknown_module_resolver_failures_for_comparison() {
        let inner_pal = PalMock::new();
        let pal = CapturingPal::new(PalHandle::new(inner_pal));
        let observed = execute_spec_example(
            &pal,
            Path::new("/tmp/spec-validation"),
            &SpecExample {
                chapter_path: FilePath::from("docs/spec/25.01 Modules - File modules.md"),
                name: SharedString::from("calling an unknown module is a resolver error"),
                source_files: vec![SpecExampleFile::new(
                    "main.ocelot-script",
                    "helper::greet();",
                )],
                expected_outcome: ExpectedOutcome::Error(SharedString::from("unknown module")),
                line_number: 24,
            },
        )
        .unwrap();

        assert_eq!(
            observed,
            ObservedOutcome::Error(SharedString::from(
                "error: unknown module `helper`\n  ╭▸ main.ocelot-script:1:1\n  │\n1 │ helper::greet();\n  ╰╴━━━━━━━━━━━━━ unknown module\nat main.ocelot-script:1:1"
            ))
        );
    }

    #[test]
    fn normalizes_duplicate_import_failures_for_comparison() {
        let inner_pal = PalMock::new();
        let pal = CapturingPal::new(PalHandle::new(inner_pal));
        let observed = execute_spec_example(
            &pal,
            Path::new("/tmp/spec-validation"),
            &SpecExample {
                chapter_path: FilePath::from("docs/spec/25.02 Modules - Imports.md"),
                name: SharedString::from("duplicate imports are a resolver error"),
                source_files: vec![
                    SpecExampleFile::new(
                        "main.ocelot-script",
                        "use helper::greet;\nuse helper::greet;",
                    ),
                    SpecExampleFile::new("helper.ocelot", "fun greet() {}"),
                ],
                expected_outcome: ExpectedOutcome::Error(SharedString::from("duplicate import")),
                line_number: 10,
            },
        )
        .unwrap();

        assert_eq!(
            observed,
            ObservedOutcome::Error(SharedString::from(
                "error: duplicate import `greet`\n  ╭▸ main.ocelot-script:2:13\n  │\n2 │ use helper::greet;\n  ╰╴            ━━━━━ duplicate import\nat main.ocelot-script:2:13"
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
    fn skips_format_validation_for_unparseable_examples() {
        let inner_pal = PalMock::new();
        let pal = CapturingPal::new(PalHandle::new(inner_pal));
        let observed = execute_spec_example(
            &pal,
            Path::new("/tmp/spec-validation"),
            &SpecExample {
                chapter_path: FilePath::from(
                    "docs/spec/91.01 Lexer errors - Unterminated strings.md",
                ),
                name: SharedString::from("unterminated string"),
                source_files: vec![SpecExampleFile::new(
                    "main.ocelot-script",
                    "println(\"hello);",
                )],
                expected_outcome: ExpectedOutcome::Error(SharedString::from(
                    "error: unterminated string literal",
                )),
                line_number: 10,
            },
        )
        .unwrap();

        let ObservedOutcome::Error(actual) = observed else {
            panic!("expected error outcome");
        };
        assert!(actual.as_str().contains("unterminated string literal"));
    }
}
