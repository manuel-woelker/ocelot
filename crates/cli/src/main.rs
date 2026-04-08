use ocelot_base::cli::format_cli_error;
use ocelot_base::error::OcelotError;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_base::source_file::SourceFile;
use ocelot_engine::discovered_test::DiscoveredTest;
use ocelot_engine::engine::Engine;
use ocelot_engine::failed_test_result::FailedTestResult;
use ocelot_engine::test_filter::TestFilter;
use ocelot_engine::test_run_summary::TestRunSummary;
use ocelot_formatter::format_compilation_unit::format_compilation_unit;
use ocelot_pal::pal::PalHandle;
use ocelot_pal::pal_real::PalReal;
use ocelot_parser::parse_compilation_unit::parse_compilation_unit;
use std::ffi::OsString;
use std::process::ExitCode;

#[derive(Debug, PartialEq, Eq)]
enum CliCommand {
    Fmt,
    Run { path: String },
    Test { filter_expression: Option<String> },
}

fn main() -> ExitCode {
    let pal = match PalReal::new_handle() {
        Ok(pal) => pal,
        Err(error) => {
            eprint!("{}", format_cli_error("operation failed", &error));
            return ExitCode::FAILURE;
        }
    };

    run_command(pal)
}

fn run_command(pal: PalHandle) -> ExitCode {
    let Some(command) = parse_command(pal.args()) else {
        print_usage();
        return ExitCode::FAILURE;
    };

    execute_command(pal, &command)
}

fn parse_command(args: Vec<OsString>) -> Option<CliCommand> {
    let mut args = args.into_iter().map(os_string_to_string);
    let first_arg = args.next()?;

    match first_arg.as_str() {
        "fmt" if args.next().is_none() => Some(CliCommand::Fmt),
        "fmt" => None,
        "test" => parse_test_command(args.collect()),
        path => Some(CliCommand::Run {
            path: path.to_owned(),
        }),
    }
}

fn parse_test_command(args: Vec<String>) -> Option<CliCommand> {
    match args.as_slice() {
        [] => Some(CliCommand::Test {
            filter_expression: None,
        }),
        [filter_expression] => Some(CliCommand::Test {
            filter_expression: Some(filter_expression.clone()),
        }),
        _ => None,
    }
}

fn os_string_to_string(value: OsString) -> String {
    value.to_string_lossy().into_owned()
}

fn execute_command(pal: PalHandle, command: &CliCommand) -> ExitCode {
    match try_execute_command(pal, command) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprint!("{}", format_cli_error("operation failed", &error));
            ExitCode::FAILURE
        }
    }
}

fn try_execute_command(pal: PalHandle, command: &CliCommand) -> OcelotResult<ExitCode> {
    let engine = Engine::new(pal.clone());

    match command {
        CliCommand::Fmt => {
            format_current_directory(&pal)?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Run { path } => {
            engine.run_file(path.as_str())?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Test { filter_expression } => {
            let summary = run_test_files(&engine, &pal, filter_expression.as_deref())?;
            report_test_summary(&summary);
            Ok(exit_code_for_test_summary(&summary))
        }
    }
}

fn format_current_directory(pal: &PalHandle) -> OcelotResult<()> {
    for path in discover_ocelot_files(pal)? {
        format_file(pal, &path)?;
    }

    Ok(())
}

fn format_file(pal: &PalHandle, path: &FilePath) -> OcelotResult<()> {
    let source = pal.read_file_to_string(path)?;
    let source_file = SourceFile::new(path.clone(), source.clone());
    let mut source_diagnostics = SourceDiagnostics::default();
    let compilation_unit = parse_compilation_unit(&source_file, &mut source_diagnostics)?;
    let formatted = format_compilation_unit(&compilation_unit);

    if formatted == source.as_str() {
        return Ok(());
    }

    let temporary_path = formatting_temporary_path(path)?;
    pal.write_file(&temporary_path, formatted.as_bytes())?;
    pal.rename(&temporary_path, path)?;
    Ok(())
}

fn formatting_temporary_path(path: &FilePath) -> OcelotResult<FilePath> {
    let parent = path.parent().unwrap_or_default();
    let file_name = path
        .file_name()
        .ok_or_else(|| OcelotError::message(format!("path '{}' has no file name", path)))?;
    Ok(parent.join(format!(".{file_name}.ocelot-fmt.tmp")))
}

fn run_test_files(
    engine: &Engine,
    pal: &PalHandle,
    filter_expression: Option<&str>,
) -> OcelotResult<TestRunSummary> {
    let script_paths = discover_ocelot_files(pal)?;
    let test_filter = parse_test_filter(filter_expression);

    let mut summary = TestRunSummary::new();
    for script_path in script_paths {
        if test_filter.is_empty() || test_filter_matches_path(&test_filter, &script_path) {
            merge_test_summary(&mut summary, run_test_file(engine, &script_path));
            continue;
        }

        match discover_matching_test_names(engine, &script_path, &test_filter) {
            Ok(test_names) if test_names.is_empty() => {}
            Ok(test_names) => {
                merge_test_summary(
                    &mut summary,
                    run_named_test_file(engine, &script_path, test_names),
                );
            }
            Err(error) => merge_test_summary(
                &mut summary,
                file_failure_summary(&script_path, format_cli_error("test file failed", &error)),
            ),
        }
    }

    if filter_expression.is_some() && summary.passed.is_empty() && summary.failed.is_empty() {
        return Err(OcelotError::message(format!(
            "no tests matched filter expression `{}`",
            filter_expression.unwrap_or_default()
        )));
    }

    Ok(summary)
}

fn run_test_file(engine: &Engine, script_path: &FilePath) -> TestRunSummary {
    match engine.run_tests(script_path) {
        Ok(summary) => summary,
        Err(error) => {
            file_failure_summary(script_path, format_cli_error("test file failed", &error))
        }
    }
}

fn run_named_test_file(
    engine: &Engine,
    script_path: &FilePath,
    test_names: Vec<SharedString>,
) -> TestRunSummary {
    match engine.run_named_tests(script_path, test_names) {
        Ok(summary) => summary,
        Err(error) => {
            file_failure_summary(script_path, format_cli_error("test file failed", &error))
        }
    }
}

fn file_failure_summary(script_path: &FilePath, message: String) -> TestRunSummary {
    TestRunSummary {
        passed: Vec::new(),
        failed: vec![FailedTestResult::new(
            script_path.clone(),
            script_path.as_str(),
            message,
        )],
    }
}

fn parse_test_filter(filter_expression: Option<&str>) -> TestFilter {
    let parts = filter_expression
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(SharedString::from)
        .collect();
    TestFilter::new(parts)
}

fn test_filter_matches_path(test_filter: &TestFilter, script_path: &FilePath) -> bool {
    test_filter
        .parts()
        .iter()
        .any(|part| script_path.as_str().contains(part.as_str()))
}

fn discover_matching_test_names(
    engine: &Engine,
    script_path: &FilePath,
    test_filter: &TestFilter,
) -> OcelotResult<Vec<SharedString>> {
    Ok(engine
        .discover_tests(script_path)?
        .into_iter()
        .filter(|test| test_filter_matches_test(test_filter, test))
        .map(|test| test.name)
        .collect())
}

fn test_filter_matches_test(test_filter: &TestFilter, test: &DiscoveredTest) -> bool {
    test_filter.parts().iter().any(|part| {
        test.name.contains(part.as_str()) || test.file_path.as_str().contains(part.as_str())
    })
}

fn discover_ocelot_files(pal: &PalHandle) -> OcelotResult<Vec<FilePath>> {
    let mut script_paths = pal
        .walk_directory(
            &FilePath::from(""),
            &[String::from("*.ocelot"), String::from("*.ocelot-script")],
        )?
        .collect::<OcelotResult<Vec<_>>>()?;
    script_paths.retain(is_ocelot_file);
    script_paths.sort();
    Ok(script_paths)
}

fn is_ocelot_file(path: &FilePath) -> bool {
    matches!(path.extension(), Some("ocelot" | "ocelot-script"))
}

fn merge_test_summary(summary: &mut TestRunSummary, other: TestRunSummary) {
    summary.passed.extend(other.passed);
    summary.failed.extend(other.failed);
}

fn exit_code_for_test_summary(summary: &TestRunSummary) -> ExitCode {
    if summary.is_success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_usage() {
    eprintln!("Usage:\n  ocelot fmt\n  ocelot <source-file>\n  ocelot test [filter-expression]");
}

fn report_test_summary(summary: &TestRunSummary) {
    for line in render_test_summary_lines(summary) {
        println!("{line}");
    }
}

fn render_test_summary_lines(summary: &TestRunSummary) -> Vec<String> {
    let mut lines = Vec::new();

    for passed_test in &summary.passed {
        lines.push(format!("PASS {}", passed_test.name));
    }

    for failed_test in &summary.failed {
        lines.push(format!(
            "FAIL {}",
            highlight_failed_test_name(failed_test.name.as_str())
        ));
        for detail_line in render_failed_test_detail_lines(failed_test) {
            lines.push(detail_line);
        }
    }

    lines
}

fn highlight_failed_test_name(name: &str) -> String {
    format!("\u{1b}[1;31m{name}\u{1b}[0m")
}

fn render_failed_test_detail_lines(failed_test: &FailedTestResult) -> Vec<String> {
    let header = format!("test `{}` failed", failed_test.name);
    let detail = failed_test
        .message
        .strip_prefix(header.as_str())
        .and_then(|message| message.strip_prefix('\n'))
        .unwrap_or(failed_test.message.as_str());

    let mut lines = Vec::new();
    lines.push(String::new());
    lines.extend(detail.lines().map(|line| format!("  {line}")));
    lines.push(String::from("  ────────────────────────────────────────"));
    lines.push(String::new());
    lines
}

#[cfg(test)]
mod tests {
    use super::{
        CliCommand, discover_matching_test_names, parse_command, parse_test_filter,
        render_test_summary_lines, run_command, run_test_files, test_filter_matches_path,
    };
    use ocelot_base::shared_string::SharedString;
    use ocelot_engine::engine::Engine;
    use ocelot_engine::failed_test_result::FailedTestResult;
    use ocelot_engine::passed_test_result::PassedTestResult;
    use ocelot_engine::test_run_summary::TestRunSummary;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;
    use std::ffi::OsString;
    use std::process::ExitCode;

    #[test]
    fn parses_run_command() {
        assert_eq!(
            parse_command(vec![OsString::from("examples/hello.ocelot-script")]),
            Some(CliCommand::Run {
                path: "examples/hello.ocelot-script".into(),
            })
        );
    }

    #[test]
    fn parses_fmt_command() {
        assert_eq!(
            parse_command(vec![OsString::from("fmt")]),
            Some(CliCommand::Fmt)
        );
    }

    #[test]
    fn rejects_fmt_command_with_extra_arguments() {
        assert_eq!(
            parse_command(vec![
                OsString::from("fmt"),
                OsString::from("examples/main.ocelot")
            ]),
            None
        );
    }

    #[test]
    fn rejects_test_command_with_multiple_arguments() {
        assert_eq!(
            parse_command(vec![
                OsString::from("test"),
                OsString::from("smoke"),
                OsString::from("extra"),
            ]),
            None
        );
    }

    #[test]
    fn parses_test_command_without_filter_expression() {
        assert_eq!(
            parse_command(vec![OsString::from("test")]),
            Some(CliCommand::Test {
                filter_expression: None,
            })
        );
    }

    #[test]
    fn parses_test_command_with_filter_expression() {
        assert_eq!(
            parse_command(vec![OsString::from("test"), OsString::from("smoke,parser")]),
            Some(CliCommand::Test {
                filter_expression: Some("smoke,parser".into()),
            })
        );
    }

    #[test]
    fn parses_plain_script_execution() {
        let pal = PalMock::new();
        pal.set_file("examples/hello.ocelot-script", "println(\"hello\");");
        pal.set_args(["examples/hello.ocelot-script"]);

        assert_eq!(run_command(PalHandle::new(pal.clone())), ExitCode::SUCCESS);
    }

    #[test]
    fn parses_module_execution() {
        let pal = PalMock::new();
        pal.set_file("examples/tool.ocelot", "fun main() { println(\"hello\"); }");
        pal.set_args(["examples/tool.ocelot"]);

        assert_eq!(run_command(PalHandle::new(pal.clone())), ExitCode::SUCCESS);
    }

    #[test]
    fn parses_test_execution() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/tests.ocelot",
            "test \"passes\" { println(\"ok\"); }",
        );
        pal.set_args(["test", "tests"]);

        assert_eq!(run_command(PalHandle::new(pal.clone())), ExitCode::SUCCESS);
    }

    #[test]
    fn runs_all_discovered_test_files_when_no_paths_are_given() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/first.ocelot-script",
            "test \"first\" { println(\"one\"); }",
        );
        pal.set_file(
            "examples/second.ocelot",
            "test \"second\" { println(\"two\"); }",
        );
        pal.set_file("examples/notes.txt", "not a script");
        pal.set_args(["test"]);

        assert_eq!(run_command(PalHandle::new(pal.clone())), ExitCode::SUCCESS);
        let effects = pal.get_effects();
        assert!(effects.contains("READ FILE: examples/first.ocelot-script"));
        assert!(effects.contains("READ FILE: examples/second.ocelot"));
        assert_eq!(pal.take_printed_output(), "one\ntwo\n");
    }

    #[test]
    fn fmt_skips_files_that_are_already_formatted() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "println(\"hello\");");
        pal.set_args(["fmt"]);

        assert_eq!(run_command(PalHandle::new(pal.clone())), ExitCode::SUCCESS);
        let effects = pal.get_effects();
        assert!(effects.contains("READ FILE: examples/main.ocelot-script"));
        assert!(!effects.contains("WRITE FILE:"));
        assert!(!effects.contains("RENAME FILE:"));
    }

    #[test]
    fn fmt_rewrites_misformatted_files_atomically() {
        let pal = PalMock::new();
        pal.set_file("examples/main.ocelot-script", "println( \"hello\" );");
        pal.set_args(["fmt"]);

        assert_eq!(run_command(PalHandle::new(pal.clone())), ExitCode::SUCCESS);
        assert_eq!(
            pal.read_file_string("examples/main.ocelot-script")
                .as_deref(),
            Some("println(\"hello\");")
        );
        let effects = pal.get_effects();
        assert!(effects.contains(
            "WRITE FILE: examples/.main.ocelot-script.ocelot-fmt.tmp -> println(\"hello\");"
        ));
        assert!(effects.contains("RENAME FILE: examples/.main.ocelot-script.ocelot-fmt.tmp -> examples/main.ocelot-script"));
    }

    #[test]
    fn fmt_fails_when_a_file_does_not_parse() {
        let pal = PalMock::new();
        pal.set_file("examples/broken.ocelot-script", "println(\"hello);");
        pal.set_args(["fmt"]);

        assert_eq!(run_command(PalHandle::new(pal.clone())), ExitCode::FAILURE);
        assert_eq!(
            pal.read_file_string("examples/broken.ocelot-script")
                .as_deref(),
            Some("println(\"hello);")
        );
        let effects = pal.get_effects();
        assert!(effects.contains("READ FILE: examples/broken.ocelot-script"));
        assert!(!effects.contains("WRITE FILE:"));
        assert!(!effects.contains("RENAME FILE:"));
    }

    #[test]
    fn runs_only_tests_with_matching_names() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/first.ocelot-script",
            "test \"smoke first\" { println(\"one\"); }",
        );
        pal.set_file(
            "examples/second.ocelot",
            "test \"other\" { println(\"two\"); }",
        );
        pal.set_args(["test", "smoke"]);

        assert_eq!(run_command(PalHandle::new(pal.clone())), ExitCode::SUCCESS);
        let effects = pal.get_effects();
        assert!(effects.contains("READ FILE: examples/first.ocelot-script"));
        assert!(effects.contains("READ FILE: examples/second.ocelot"));
        assert_eq!(pal.take_printed_output(), "one\n");
    }

    #[test]
    fn runs_all_tests_in_files_with_matching_paths() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/smoke_suite.ocelot-script",
            "test \"first\" { println(\"one\"); }\ntest \"second\" { println(\"two\"); }",
        );
        pal.set_file(
            "examples/other.ocelot",
            "test \"third\" { println(\"three\"); }",
        );
        pal.set_args(["test", "smoke_suite"]);

        assert_eq!(run_command(PalHandle::new(pal.clone())), ExitCode::SUCCESS);
        assert_eq!(pal.take_printed_output(), "one\ntwo\n");
    }

    #[test]
    fn collects_parse_errors_without_stopping_other_test_roots() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/good/good.ocelot-script",
            "test \"passes\" { println(\"ok\"); }",
        );
        pal.set_file("examples/bad/bad.ocelot", "println(\"hello);");
        let pal = PalHandle::new(pal.clone());
        let engine = Engine::new(pal.clone());

        let summary = run_test_files(&engine, &pal, None).unwrap();

        assert_eq!(
            summary.passed.as_slice(),
            [PassedTestResult::new(
                "examples/good/good.ocelot-script",
                "passes"
            )]
        );
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].name, "examples/bad/bad.ocelot");
        assert_eq!(
            summary.failed[0].file_path.as_str(),
            "examples/bad/bad.ocelot"
        );
        assert!(
            summary.failed[0]
                .message
                .contains("unterminated string literal")
        );
        assert!(
            !summary.failed[0]
                .message
                .contains("crates/parser/src/parse_compilation_unit.rs")
        );
    }

    #[test]
    fn parses_arguments_from_pal() {
        let pal = PalMock::new();
        pal.set_args(["examples/hello.ocelot-script"]);
        pal.set_file("examples/hello.ocelot-script", "println(\"hello\");");

        assert_eq!(run_command(PalHandle::new(pal.clone())), ExitCode::SUCCESS);
    }

    #[test]
    fn renders_pass_and_fail_lines_for_all_tests() {
        let summary = TestRunSummary {
            passed: vec![
                PassedTestResult::new("examples/first.ocelot-script", "first"),
                PassedTestResult::new("examples/third.ocelot", "third"),
            ],
            failed: vec![FailedTestResult::new(
                "examples/broken.ocelot-script",
                "broken",
                "test `broken` failed\nerror: assert_eq values differ\n\nexpected: \"a\"\nactual:   \"b\"",
            )],
        };

        assert_eq!(
            render_test_summary_lines(&summary),
            vec![
                String::from("PASS first"),
                String::from("PASS third"),
                String::from("FAIL \u{1b}[1;31mbroken\u{1b}[0m"),
                String::new(),
                String::from("  error: assert_eq values differ"),
                String::from("  "),
                String::from("  expected: \"a\""),
                String::from("  actual:   \"b\""),
                String::from("  ────────────────────────────────────────"),
                String::new(),
            ]
        );
    }

    #[test]
    fn parses_test_filter_parts_and_ignores_empty_entries() {
        let expected: Vec<SharedString> = vec!["smoke".into(), "parser".into()];

        assert_eq!(
            parse_test_filter(Some(" smoke , , parser,, ")).parts(),
            expected.as_slice()
        );
    }

    #[test]
    fn matches_filter_parts_against_file_paths() {
        let filter = parse_test_filter(Some("smoke,parser"));

        assert!(test_filter_matches_path(
            &filter,
            &"examples/smoke_suite.ocelot-script".into()
        ));
        assert!(!test_filter_matches_path(
            &filter,
            &"examples/release_suite.ocelot-script".into()
        ));
    }

    #[test]
    fn discovers_matching_test_names_only() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/tests.ocelot-script",
            "test \"smoke first\" { println(\"one\"); }\ntest \"other\" { println(\"two\"); }",
        );
        let engine = Engine::new(PalHandle::new(pal));

        assert_eq!(
            discover_matching_test_names(
                &engine,
                &"examples/tests.ocelot-script".into(),
                &parse_test_filter(Some("smoke"))
            )
            .unwrap(),
            vec![SharedString::from("smoke first")]
        );
    }

    #[test]
    fn filtered_runs_fail_when_no_tests_match() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/tests.ocelot-script",
            "test \"passes\" { println(\"ok\"); }",
        );
        let pal_handle = PalHandle::new(pal.clone());
        let engine = Engine::new(pal_handle.clone());

        let error = run_test_files(&engine, &pal_handle, Some("missing")).unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("no tests matched filter expression `missing`")
        );
    }
}
