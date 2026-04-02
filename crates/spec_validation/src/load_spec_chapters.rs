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
    let mut output_block: Option<String> = None;
    let mut waiting_for_output_block = false;
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
                &mut output_block,
                example_line_number,
            );
            example_name = Some(rest.trim().to_owned());
            example_line_number = line_index + 1;
            waiting_for_output_block = false;
            line_index += 1;
            continue;
        }

        if example_name.is_none() {
            line_index += 1;
            continue;
        }

        if trimmed == "### Output" {
            waiting_for_output_block = true;
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
            waiting_for_output_block = false;
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

            if !waiting_for_output_block {
                malformed_failures.push(new_malformed_failure(
                    path,
                    example_name.as_deref().unwrap_or("unknown example"),
                    example_line_number,
                    "`text` block must appear under `### Output`",
                ));
            } else if output_block.is_some() {
                malformed_failures.push(new_malformed_failure(
                    path,
                    example_name.as_deref().unwrap_or("unknown example"),
                    example_line_number,
                    "example must contain exactly one output block",
                ));
            } else {
                output_block = Some(normalize_validation_text(
                    &lines[line_index + 1..block_end].join("\n"),
                ));
            }
            waiting_for_output_block = false;
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
        &mut output_block,
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
    output_block: &mut Option<String>,
    example_line_number: usize,
) {
    let Some(name) = example_name.take() else {
        return;
    };

    match (source_block.take(), output_block.take()) {
        (Some(source), Some(expected_output)) => examples.push(SpecExample {
            chapter_path: path.clone(),
            name: SharedString::from(name),
            source: SharedString::from(source),
            expected_output: SharedString::from(expected_output),
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
            "example is missing its `### Output` `text` block",
        )),
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

        expect!["example is missing its `### Output` `text` block"]
            .assert_eq(malformed.message.as_str());
        assert_eq!(malformed.line_number, 2);
    }

    fn chapter(name: &str) -> String {
        format!(
            "## Example: {name}\n\n```ocelot\nprintln(\"hello\");\n```\n\n### Output\n\n```text\nhello\n```\n"
        )
    }
}
