use crate::expected_outcome::ExpectedOutcome;
use crate::loaded_spec_chapter::LoadedSpecChapter;
use crate::normalize_validation_text::normalize_validation_text;
use crate::spec_example::SpecExample;
use crate::validation_failure::ValidationFailure;
use crate::validation_failure_kind::ValidationFailureKind;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_pal::pal::Pal;

/// Loads numbered spec chapters from markdown files in filename order.
pub fn load_spec_chapters(
    pal: &dyn Pal,
    spec_root: &FilePath,
) -> OcelotResult<Vec<LoadedSpecChapter>> {
    let mut chapter_paths = pal
        .walk_directory(spec_root, &[String::from("*.md")])?
        .collect::<OcelotResult<Vec<_>>>()?;
    chapter_paths.retain(is_numbered_spec_chapter);
    chapter_paths.sort();

    chapter_paths
        .into_iter()
        .map(|path| {
            let markdown = pal.read_file_to_string(&path)?;
            Ok(parse_spec_chapter(&path, markdown.as_str()))
        })
        .collect()
}

fn is_numbered_spec_chapter(path: &FilePath) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };

    let mut chars = file_name.chars();
    matches!(chars.next(), Some(a) if a.is_ascii_digit())
        && matches!(chars.next(), Some(b) if b.is_ascii_digit())
        && matches!(chars.next(), Some('.'))
        && matches!(chars.next(), Some(c) if c.is_ascii_digit())
        && matches!(chars.next(), Some(d) if d.is_ascii_digit())
}

fn parse_spec_chapter(path: &FilePath, markdown: &str) -> LoadedSpecChapter {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut examples = Vec::new();
    let mut malformed_failures = Vec::new();

    let mut example_name: Option<String> = None;
    let mut example_line_number = 0usize;
    let mut source_block: Option<String> = None;
    let mut expected_outcome: Option<ExpectedOutcome> = None;
    let mut waiting_for_expectation = None;
    let mut line_index = 0usize;

    while line_index < lines.len() {
        let line = lines[line_index];
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("## Example:") {
            finalize_example(
                path,
                &mut examples,
                &mut malformed_failures,
                &mut example_name,
                &mut source_block,
                &mut expected_outcome,
                example_line_number,
            );
            example_name = Some(rest.trim().to_owned());
            example_line_number = line_index + 1;
            waiting_for_expectation = None;
            line_index += 1;
            continue;
        }

        if example_name.is_none() {
            line_index += 1;
            continue;
        }

        if let Some(expectation_kind) = parse_expectation_heading(trimmed) {
            if waiting_for_expectation.is_some() || expected_outcome.is_some() {
                malformed_failures.push(new_malformed_failure(
                    path,
                    example_name.as_deref().unwrap_or("unknown example"),
                    example_line_number,
                    "example must contain exactly one `### Output` or `### Error` section",
                ));
            } else {
                waiting_for_expectation = Some(expectation_kind);
            }
            line_index += 1;
            continue;
        }

        if trimmed == "```ocelot" {
            let Some(block_end) = find_closing_fence(&lines, line_index + 1) else {
                malformed_failures.push(new_malformed_failure(
                    path,
                    example_name.as_deref().unwrap_or("unknown example"),
                    example_line_number,
                    "missing closing fence for `ocelot` block",
                ));
                break;
            };

            if source_block.is_some() {
                malformed_failures.push(new_malformed_failure(
                    path,
                    example_name.as_deref().unwrap_or("unknown example"),
                    example_line_number,
                    "example must contain exactly one `ocelot` block",
                ));
            } else {
                source_block = Some(lines[line_index + 1..block_end].join("\n"));
            }
            waiting_for_expectation = None;
            line_index = block_end + 1;
            continue;
        }

        if trimmed == "```text" {
            let Some(block_end) = find_closing_fence(&lines, line_index + 1) else {
                malformed_failures.push(new_malformed_failure(
                    path,
                    example_name.as_deref().unwrap_or("unknown example"),
                    example_line_number,
                    "missing closing fence for `text` block",
                ));
                break;
            };

            let Some(expectation_kind) = waiting_for_expectation.take() else {
                malformed_failures.push(new_malformed_failure(
                    path,
                    example_name.as_deref().unwrap_or("unknown example"),
                    example_line_number,
                    "`text` block must appear under `### Output` or `### Error`",
                ));
                line_index = block_end + 1;
                continue;
            };

            if expected_outcome.is_some() {
                malformed_failures.push(new_malformed_failure(
                    path,
                    example_name.as_deref().unwrap_or("unknown example"),
                    example_line_number,
                    "example must contain exactly one expectation block",
                ));
            } else {
                let text = SharedString::from(normalize_validation_text(
                    &lines[line_index + 1..block_end].join("\n"),
                ));
                expected_outcome = Some(match expectation_kind {
                    ExpectationKind::Output => ExpectedOutcome::Output(text),
                    ExpectationKind::Error => ExpectedOutcome::Error(text),
                });
            }
            line_index = block_end + 1;
            continue;
        }

        line_index += 1;
    }

    finalize_example(
        path,
        &mut examples,
        &mut malformed_failures,
        &mut example_name,
        &mut source_block,
        &mut expected_outcome,
        example_line_number,
    );

    LoadedSpecChapter {
        path: path.clone(),
        examples,
        malformed_failures,
    }
}

fn finalize_example(
    path: &FilePath,
    examples: &mut Vec<SpecExample>,
    malformed_failures: &mut Vec<ValidationFailure>,
    example_name: &mut Option<String>,
    source_block: &mut Option<String>,
    expected_outcome: &mut Option<ExpectedOutcome>,
    example_line_number: usize,
) {
    let Some(name) = example_name.take() else {
        return;
    };

    match (source_block.take(), expected_outcome.take()) {
        (Some(source), Some(expected_outcome)) => examples.push(SpecExample {
            chapter_path: path.clone(),
            name: SharedString::from(name),
            source: SharedString::from(source),
            expected_outcome,
            line_number: example_line_number,
        }),
        (None, _) => malformed_failures.push(new_malformed_failure(
            path,
            &name,
            example_line_number,
            "example is missing its fenced `ocelot` block",
        )),
        (_, None) => malformed_failures.push(new_malformed_failure(
            path,
            &name,
            example_line_number,
            "example is missing its `### Output` or `### Error` `text` block",
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectationKind {
    Output,
    Error,
}

fn parse_expectation_heading(trimmed_line: &str) -> Option<ExpectationKind> {
    match trimmed_line {
        "### Output" => Some(ExpectationKind::Output),
        "### Error" => Some(ExpectationKind::Error),
        _ => None,
    }
}

fn find_closing_fence(lines: &[&str], start_index: usize) -> Option<usize> {
    (start_index..lines.len()).find(|line_index| lines[*line_index].trim() == "```")
}

fn new_malformed_failure(
    chapter_path: &FilePath,
    example_name: &str,
    line_number: usize,
    message: &str,
) -> ValidationFailure {
    ValidationFailure {
        chapter_path: chapter_path.clone(),
        example_name: SharedString::from(example_name),
        kind: ValidationFailureKind::MalformedExample,
        message: SharedString::from(message),
        expected_output: None,
        actual_output: None,
        line_number,
    }
}

#[cfg(test)]
mod tests {
    use super::load_spec_chapters;
    use crate::expected_outcome::ExpectedOutcome;
    use expect_test::expect;
    use ocelot_base::file_path::FilePath;
    use ocelot_pal::pal_mock::PalMock;

    #[test]
    fn loads_numbered_spec_chapters_in_filename_order() {
        let pal = PalMock::new();
        pal.set_file(
            "docs/spec/09.01 Standard library - println.md",
            chapter("later"),
        );
        pal.set_file(
            "docs/spec/08.01 Runtime behavior - Scripts.md",
            chapter("earlier"),
        );
        pal.set_file("docs/spec/README.md", "# ignored");

        let chapters = load_spec_chapters(&pal, &FilePath::from("docs/spec")).unwrap();

        assert_eq!(chapters.len(), 2);
        assert_eq!(
            chapters
                .iter()
                .map(|chapter| chapter.path.as_str())
                .collect::<Vec<_>>(),
            vec![
                "docs/spec/08.01 Runtime behavior - Scripts.md",
                "docs/spec/09.01 Standard library - println.md",
            ]
        );
        assert_eq!(chapters[0].examples[0].name.as_str(), "earlier");
        assert_eq!(
            chapters[0].examples[0].expected_outcome,
            ExpectedOutcome::Output("hello".into())
        );
    }

    #[test]
    fn reports_malformed_examples() {
        let pal = PalMock::new();
        pal.set_file(
            "docs/spec/08.01 Runtime behavior - Scripts.md",
            r#"
## Example: broken

```ocelot
println("hello");
```
"#,
        );

        let chapters = load_spec_chapters(&pal, &FilePath::from("docs/spec")).unwrap();
        let malformed = &chapters[0].malformed_failures[0];

        expect!["example is missing its `### Output` or `### Error` `text` block"]
            .assert_eq(malformed.message.as_str());
        assert_eq!(malformed.line_number, 2);
    }

    #[test]
    fn extracts_error_expectations() {
        let pal = PalMock::new();
        pal.set_file(
            "docs/spec/09.01 Standard library - println.md",
            r#"
## Example: fails

```ocelot
println();
```

### Error

```text
type error
```
"#,
        );

        let chapters = load_spec_chapters(&pal, &FilePath::from("docs/spec")).unwrap();

        assert_eq!(
            chapters[0].examples[0].expected_outcome,
            ExpectedOutcome::Error("type error".into())
        );
    }

    #[test]
    fn reports_examples_with_both_expectation_sections_as_malformed() {
        let pal = PalMock::new();
        pal.set_file(
            "docs/spec/09.01 Standard library - println.md",
            r#"
## Example: ambiguous

```ocelot
println("hello");
```

### Output

```text
hello
```

### Error

```text
boom
```
"#,
        );

        let chapters = load_spec_chapters(&pal, &FilePath::from("docs/spec")).unwrap();

        expect!["example must contain exactly one `### Output` or `### Error` section"]
            .assert_eq(chapters[0].malformed_failures[0].message.as_str());
    }

    fn chapter(name: &str) -> String {
        format!(
            "## Example: {name}\n\n```ocelot\nprintln(\"hello\");\n```\n\n### Output\n\n```text\nhello\n```\n"
        )
    }
}
