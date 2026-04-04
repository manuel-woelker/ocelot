use ocelot_base::cli::format_cli_error;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_engine::engine::Engine;
use ocelot_engine::failed_test_result::FailedTestResult;
use ocelot_engine::test_run_summary::TestRunSummary;
use ocelot_pal::pal::PalHandle;
use ocelot_pal::pal_real::PalReal;
use std::ffi::OsString;
use std::process::ExitCode;

#[derive(Debug, PartialEq, Eq)]
enum CliCommand {
    Run { path: String },
    Test { script_paths: Vec<String> },
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
        "test" => Some(CliCommand::Test {
            script_paths: args.collect(),
        }),
        path => Some(CliCommand::Run {
            path: path.to_owned(),
        }),
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
        CliCommand::Run { path } => {
            engine.run_file(path.as_str())?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Test { script_paths } => {
            let summary = run_test_files(&engine, &pal, script_paths)?;
            report_test_summary(&summary);
            Ok(exit_code_for_test_summary(&summary))
        }
    }
}

fn run_test_files(
    engine: &Engine,
    pal: &PalHandle,
    script_paths: &[String],
) -> OcelotResult<TestRunSummary> {
    let script_paths = if script_paths.is_empty() {
        discover_ocelot_files(pal)?
    } else {
        script_paths
            .iter()
            .map(|path| path.as_str().into())
            .collect()
    };

    let mut summary = TestRunSummary::new();
    for script_path in script_paths {
        merge_test_summary(&mut summary, run_test_file(engine, &script_path));
    }

    Ok(summary)
}

fn run_test_file(engine: &Engine, script_path: &FilePath) -> TestRunSummary {
    match engine.run_tests(script_path) {
        Ok(summary) => summary,
        Err(error) => TestRunSummary {
            passed: Vec::new(),
            failed: vec![FailedTestResult::new(
                script_path.as_str(),
                format_cli_error("test file failed", &error),
            )],
        },
    }
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
    eprintln!("Usage:\n  ocelot <source-file>\n  ocelot test [source-file...]");
}

fn report_test_summary(summary: &TestRunSummary) {
    for line in render_test_summary_lines(summary) {
        println!("{line}");
    }
}

fn render_test_summary_lines(summary: &TestRunSummary) -> Vec<String> {
    let mut lines = Vec::new();

    for test_name in &summary.passed {
        lines.push(format!("PASS {test_name}"));
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
        CliCommand, parse_command, render_test_summary_lines, run_command, run_test_files,
    };
    use ocelot_engine::engine::Engine;
    use ocelot_engine::failed_test_result::FailedTestResult;
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
    fn parses_test_command_with_multiple_paths() {
        assert_eq!(
            parse_command(vec![
                OsString::from("test"),
                OsString::from("examples/first.ocelot-script"),
                OsString::from("examples/second.ocelot"),
            ]),
            Some(CliCommand::Test {
                script_paths: vec![
                    "examples/first.ocelot-script".into(),
                    "examples/second.ocelot".into()
                ],
            })
        );
    }

    #[test]
    fn parses_test_command_without_paths() {
        assert_eq!(
            parse_command(vec![OsString::from("test")]),
            Some(CliCommand::Test {
                script_paths: Vec::new(),
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
        pal.set_args(["test", "examples/tests.ocelot"]);

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
    fn runs_all_explicit_test_files() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/first.ocelot-script",
            "test \"first\" { println(\"one\"); }",
        );
        pal.set_file(
            "examples/second.ocelot",
            "test \"second\" { println(\"two\"); }",
        );
        pal.set_args([
            "test",
            "examples/first.ocelot-script",
            "examples/second.ocelot",
        ]);

        assert_eq!(run_command(PalHandle::new(pal.clone())), ExitCode::SUCCESS);
        let effects = pal.get_effects();
        assert!(effects.contains("READ FILE: examples/first.ocelot-script"));
        assert!(effects.contains("READ FILE: examples/second.ocelot"));
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

        let summary = run_test_files(
            &engine,
            &pal,
            &[
                "examples/good/good.ocelot-script".into(),
                "examples/bad/bad.ocelot".into(),
            ],
        )
        .unwrap();

        assert_eq!(summary.passed.as_slice(), ["passes"]);
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].name, "examples/bad/bad.ocelot");
        assert!(
            summary.failed[0]
                .message
                .contains("unterminated string literal")
        );
        assert!(
            !summary.failed[0]
                .message
                .contains("crates/parser/src/parse_script.rs")
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
            passed: vec!["first".into(), "third".into()],
            failed: vec![FailedTestResult::new(
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
}
