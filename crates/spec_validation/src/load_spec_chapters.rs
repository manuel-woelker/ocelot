use crate::expected_outcome::ExpectedOutcome;
use crate::loaded_spec_chapter::LoadedSpecChapter;
use crate::normalize_validation_text::normalize_validation_text;
use crate::spec_example::SpecExample;
use crate::spec_example_file::SpecExampleFile;
use crate::validation_failure::ValidationFailure;
use crate::validation_failure_kind::ValidationFailureKind;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_pal::pal::Pal;
use std::collections::HashSet;
use std::path::Path;

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

    let mut current_example: Option<(String, usize)> = None;
    let mut source_files: Vec<SpecExampleFile> = Vec::new();
    let mut saw_source_block = false;
    let mut expected_outcome: Option<ExpectedOutcome> = None;
    let mut waiting_for_expectation = None;
    let mut pending_source_label: Option<String> = None;
    let mut line_index = 0usize;

    while line_index < lines.len() {
        let line = lines[line_index];
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("## Example:") {
            finalize_example(
                path,
                &mut examples,
                &mut malformed_failures,
                &mut current_example,
                &mut source_files,
                &mut saw_source_block,
                &mut expected_outcome,
            );
            current_example = Some((rest.trim().to_owned(), line_index + 1));
            waiting_for_expectation = None;
            pending_source_label = None;
            saw_source_block = false;
            line_index += 1;
            continue;
        }

        if current_example.is_none() {
            line_index += 1;
            continue;
        }

        if let Some(expectation_kind) = parse_expectation_heading(trimmed) {
            if waiting_for_expectation.is_some() || expected_outcome.is_some() {
                malformed_failures.push(new_malformed_failure(
                    path,
                    current_example
                        .as_ref()
                        .map(|(name, _)| name.as_str())
                        .unwrap_or("unknown example"),
                    current_example
                        .as_ref()
                        .map(|(_, line_number)| *line_number)
                        .unwrap_or_default(),
                    "example must contain exactly one `### Output` or `### Error` section",
                ));
            } else {
                waiting_for_expectation = Some(expectation_kind);
            }
            line_index += 1;
            continue;
        }

        if let Some(file_name) = parse_source_file_label(trimmed) {
            pending_source_label = Some(file_name.to_owned());
            line_index += 1;
            continue;
        }

        if trimmed == "```ocelot" {
            saw_source_block = true;
            let Some(block_end) = find_closing_fence(&lines, line_index + 1) else {
                malformed_failures.push(new_malformed_failure(
                    path,
                    current_example
                        .as_ref()
                        .map(|(name, _)| name.as_str())
                        .unwrap_or("unknown example"),
                    current_example
                        .as_ref()
                        .map(|(_, line_number)| *line_number)
                        .unwrap_or_default(),
                    "missing closing fence for `ocelot` block",
                ));
                break;
            };

            let Some(file_name) = pending_source_label.take() else {
                malformed_failures.push(new_malformed_failure(
                    path,
                    current_example
                        .as_ref()
                        .map(|(name, _)| name.as_str())
                        .unwrap_or("unknown example"),
                    current_example
                        .as_ref()
                        .map(|(_, line_number)| *line_number)
                        .unwrap_or_default(),
                    "`ocelot` blocks must have a preceding `path/to/file.ocelot:` or `path/to/file.ocelot-script:` label",
                ));
                line_index = block_end + 1;
                continue;
            };

            source_files.push(SpecExampleFile::new(
                file_name,
                lines[line_index + 1..block_end].join("\n"),
            ));
            waiting_for_expectation = None;
            line_index = block_end + 1;
            continue;
        }

        if trimmed == "```text" {
            let Some(block_end) = find_closing_fence(&lines, line_index + 1) else {
                malformed_failures.push(new_malformed_failure(
                    path,
                    current_example
                        .as_ref()
                        .map(|(name, _)| name.as_str())
                        .unwrap_or("unknown example"),
                    current_example
                        .as_ref()
                        .map(|(_, line_number)| *line_number)
                        .unwrap_or_default(),
                    "missing closing fence for `text` block",
                ));
                break;
            };

            let Some(expectation_kind) = waiting_for_expectation.take() else {
                malformed_failures.push(new_malformed_failure(
                    path,
                    current_example
                        .as_ref()
                        .map(|(name, _)| name.as_str())
                        .unwrap_or("unknown example"),
                    current_example
                        .as_ref()
                        .map(|(_, line_number)| *line_number)
                        .unwrap_or_default(),
                    "`text` block must appear under `### Output` or `### Error`",
                ));
                line_index = block_end + 1;
                continue;
            };

            if expected_outcome.is_some() {
                malformed_failures.push(new_malformed_failure(
                    path,
                    current_example
                        .as_ref()
                        .map(|(name, _)| name.as_str())
                        .unwrap_or("unknown example"),
                    current_example
                        .as_ref()
                        .map(|(_, line_number)| *line_number)
                        .unwrap_or_default(),
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
        &mut current_example,
        &mut source_files,
        &mut saw_source_block,
        &mut expected_outcome,
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
    current_example: &mut Option<(String, usize)>,
    source_files: &mut Vec<SpecExampleFile>,
    saw_source_block: &mut bool,
    expected_outcome: &mut Option<ExpectedOutcome>,
) {
    let Some((name, example_line_number)) = current_example.take() else {
        return;
    };

    match (
        std::mem::take(source_files),
        *saw_source_block,
        expected_outcome.take(),
    ) {
        (example_source_files, false, Some(_expected_outcome))
            if example_source_files.is_empty() =>
        {
            malformed_failures.push(new_malformed_failure(
                path,
                &name,
                example_line_number,
                "example is missing its fenced `ocelot` block",
            ))
        }
        (example_source_files, _, Some(expected_outcome)) => {
            let mut seen_paths = HashSet::new();
            let duplicate_path = example_source_files.iter().find_map(|source_file| {
                (!seen_paths.insert(source_file.path.clone())).then(|| source_file.path.clone())
            });

            if let Some(duplicate_path) = duplicate_path {
                malformed_failures.push(new_malformed_failure(
                    path,
                    &name,
                    example_line_number,
                    format!("example declares duplicate source file `{duplicate_path}`").as_str(),
                ));
                return;
            }

            let has_script_entry = example_source_files
                .iter()
                .any(|source_file| source_file.path.as_str() == "main.ocelot-script");
            let has_module_entry = example_source_files
                .iter()
                .any(|source_file| source_file.path.as_str() == "main.ocelot");

            if has_script_entry && has_module_entry {
                malformed_failures.push(new_malformed_failure(
                    path,
                    &name,
                    example_line_number,
                    "example must not declare both `main.ocelot` and `main.ocelot-script`",
                ));
                return;
            }

            if !has_script_entry && !has_module_entry {
                malformed_failures.push(new_malformed_failure(
                    path,
                    &name,
                    example_line_number,
                    "example must declare `main.ocelot` or `main.ocelot-script`",
                ));
                return;
            }

            examples.push(SpecExample {
                chapter_path: path.clone(),
                name: SharedString::from(name),
                source_files: example_source_files,
                expected_outcome,
                line_number: example_line_number,
            });
        }
        (_, _, None) => malformed_failures.push(new_malformed_failure(
            path,
            &name,
            example_line_number,
            "example is missing its `### Output` or `### Error` `text` block",
        )),
    }
    *saw_source_block = false;
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

fn parse_source_file_label(trimmed_line: &str) -> Option<&str> {
    let path = trimmed_line.strip_suffix(':')?;
    matches!(
        Path::new(path).extension().and_then(|ext| ext.to_str()),
        Some("ocelot" | "ocelot-script")
    )
    .then_some(path)
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
            "docs/spec/30.01 Standard library - println.md",
            chapter("later"),
        );
        pal.set_file(
            "docs/spec/28.01 Runtime behavior - Scripts.md",
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
                "docs/spec/28.01 Runtime behavior - Scripts.md",
                "docs/spec/30.01 Standard library - println.md",
            ]
        );
        assert_eq!(chapters[0].examples[0].name.as_str(), "earlier");
        assert_eq!(
            chapters[0].examples[0].expected_outcome,
            ExpectedOutcome::Output("hello".into())
        );
        assert_eq!(
            chapters[0].examples[0].source_files[0].path.as_str(),
            "main.ocelot-script"
        );
    }

    #[test]
    fn reports_malformed_examples_without_source_files() {
        let pal = PalMock::new();
        pal.set_file(
            "docs/spec/28.01 Runtime behavior - Scripts.md",
            r#"
## Example: broken

### Output

```text
hello
```
"#,
        );

        let chapters = load_spec_chapters(&pal, &FilePath::from("docs/spec")).unwrap();
        let malformed = &chapters[0].malformed_failures[0];

        expect!["example is missing its fenced `ocelot` block"]
            .assert_eq(malformed.message.as_str());
        assert_eq!(malformed.line_number, 2);
    }

    #[test]
    fn loads_multi_file_examples() {
        let pal = PalMock::new();
        pal.set_file(
            "docs/spec/25.01 Modules - File modules.md",
            r#"
## Example: sibling module

main.ocelot-script:

```ocelot
helper::greet();
```

helper.ocelot:

```ocelot
fun greet() {
    println("hello");
}
```

### Output

```text
hello
```
"#,
        );

        let chapters = load_spec_chapters(&pal, &FilePath::from("docs/spec")).unwrap();

        assert_eq!(chapters[0].examples[0].source_files.len(), 2);
        assert_eq!(
            chapters[0].examples[0].source_files[1].path.as_str(),
            "helper.ocelot"
        );
    }

    #[test]
    fn reports_examples_without_filename_labels_as_malformed() {
        let pal = PalMock::new();
        pal.set_file(
            "docs/spec/30.01 Standard library - println.md",
            r#"
## Example: broken

```ocelot
println("hello");
```

### Output

```text
hello
```
"#,
        );

        let chapters = load_spec_chapters(&pal, &FilePath::from("docs/spec")).unwrap();

        expect!["`ocelot` blocks must have a preceding `path/to/file.ocelot:` or `path/to/file.ocelot-script:` label"]
            .assert_eq(chapters[0].malformed_failures[0].message.as_str());
    }

    #[test]
    fn reports_examples_without_main_ocelot_as_malformed() {
        let pal = PalMock::new();
        pal.set_file(
            "docs/spec/25.01 Modules - File modules.md",
            r#"
## Example: broken

helper.ocelot:

```ocelot
fun greet() {}
```

### Output

```text
```
"#,
        );

        let chapters = load_spec_chapters(&pal, &FilePath::from("docs/spec")).unwrap();

        expect!["example must declare `main.ocelot` or `main.ocelot-script`"]
            .assert_eq(chapters[0].malformed_failures[0].message.as_str());
    }

    fn chapter(name: &str) -> String {
        format!(
            "## Example: {name}\n\nmain.ocelot-script:\n\n```ocelot\nprintln(\"hello\");\n```\n\n### Output\n\n```text\nhello\n```\n"
        )
    }
}
