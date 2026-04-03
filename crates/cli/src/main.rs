mod cli_command;

use crate::cli_command::CliCommand;
use ocelot_base::cli::format_cli_error;
use ocelot_base::result::OcelotResult;
use ocelot_engine::engine::Engine;
use ocelot_engine::failed_test_result::FailedTestResult;
use ocelot_engine::test_run_summary::TestRunSummary;
use ocelot_pal::pal_real::PalReal;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(command) = parse_command() else {
        print_usage();
        return ExitCode::FAILURE;
    };

    match command {
        CliCommand::Run { script_path } => run_cli(script_path.as_str()),
        CliCommand::Test { script_path } => run_test_cli(script_path.as_str()),
    }
}

fn run_cli(script_path: &str) -> ExitCode {
    match run_script(script_path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprint!("{}", format_cli_error("operation failed", &error));
            ExitCode::FAILURE
        }
    }
}

fn run_script(script_path: &str) -> OcelotResult<()> {
    let pal = PalReal::new_handle()?;
    Engine::new(pal).run_script(script_path)
}

fn run_test_cli(script_path: &str) -> ExitCode {
    match run_tests(script_path) {
        Ok(summary) => {
            report_test_summary(&summary);
            if summary.is_success() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprint!("{}", format_cli_error("operation failed", &error));
            ExitCode::FAILURE
        }
    }
}

fn run_tests(script_path: &str) -> OcelotResult<TestRunSummary> {
    let pal = PalReal::new_handle()?;
    Engine::new(pal).run_tests(script_path)
}

fn parse_command() -> Option<CliCommand> {
    parse_command_inner(std::env::args().skip(1).collect())
}

fn print_usage() {
    eprintln!("Usage:\n  ocelot <script-file>\n  ocelot test <script-file>");
}

fn report_test_summary(summary: &TestRunSummary) {
    for line in render_test_summary_lines(summary) {
        println!("{line}");
    }
}

fn parse_command_inner(args: Vec<String>) -> Option<CliCommand> {
    match args.as_slice() {
        [script_path] => Some(CliCommand::Run {
            script_path: script_path.clone(),
        }),
        [command, script_path] if command == "test" => Some(CliCommand::Test {
            script_path: script_path.clone(),
        }),
        _ => None,
    }
}

fn render_test_summary_lines(summary: &TestRunSummary) -> Vec<String> {
    let mut lines = Vec::new();

    for test_name in &summary.passed {
        lines.push(format!("PASS {test_name}"));
    }

    for failed_test in &summary.failed {
        lines.push(format!("FAIL {}", failed_test.name));
        for detail_line in render_failed_test_detail_lines(failed_test) {
            lines.push(detail_line);
        }
    }

    lines
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
    lines
}

#[cfg(test)]
mod tests {
    use super::{parse_command_inner, render_test_summary_lines};
    use crate::cli_command::CliCommand;
    use ocelot_engine::failed_test_result::FailedTestResult;
    use ocelot_engine::test_run_summary::TestRunSummary;

    fn parse_command_from(args: &[&str]) -> Option<CliCommand> {
        parse_command_inner(args.iter().map(|value| value.to_string()).collect())
    }

    #[test]
    fn parses_plain_script_execution() {
        assert_eq!(
            parse_command_from(&["examples/hello.ocelot"]),
            Some(CliCommand::Run {
                script_path: String::from("examples/hello.ocelot"),
            })
        );
    }

    #[test]
    fn parses_test_execution() {
        assert_eq!(
            parse_command_from(&["test", "examples/tests.ocelot"]),
            Some(CliCommand::Test {
                script_path: String::from("examples/tests.ocelot"),
            })
        );
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
                String::from("FAIL broken"),
                String::new(),
                String::from("  error: assert_eq values differ"),
                String::from("  "),
                String::from("  expected: \"a\""),
                String::from("  actual:   \"b\""),
            ]
        );
    }
}
