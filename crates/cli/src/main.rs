mod cli_command;

use crate::cli_command::CliCommand;
use ocelot_base::cli::try_main;
use ocelot_base::error::OcelotError;
use ocelot_base::result::OcelotResult;
use ocelot_engine::engine::Engine;
use ocelot_engine::test_run_summary::TestRunSummary;
use ocelot_pal::pal_real::PalReal;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(command) = parse_command() else {
        print_usage();
        return ExitCode::FAILURE;
    };

    try_main(|| run_cli(&command))
}

fn run_cli(command: &CliCommand) -> OcelotResult<()> {
    let pal = PalReal::new_handle()?;
    let engine = Engine::new(pal);

    match command {
        CliCommand::Run { script_path } => engine.run_script(script_path.as_str()),
        CliCommand::Test { script_path } => {
            let summary = engine.run_tests(script_path.as_str())?;
            report_test_summary(&summary);

            if summary.is_success() {
                Ok(())
            } else {
                Err(OcelotError::message(format!(
                    "{} test(s) failed",
                    summary.failed.len()
                )))
            }
        }
    }
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
            failed: vec![FailedTestResult::new("broken", "test `broken` failed")],
        };

        assert_eq!(
            render_test_summary_lines(&summary),
            vec![
                String::from("PASS first"),
                String::from("PASS third"),
                String::from("FAIL broken"),
            ]
        );
    }
}

fn render_test_summary_lines(summary: &TestRunSummary) -> Vec<String> {
    let mut lines = Vec::new();

    for test_name in &summary.passed {
        lines.push(format!("PASS {test_name}"));
    }

    for failed_test in &summary.failed {
        lines.push(format!("FAIL {}", failed_test.name));
    }

    lines
}
