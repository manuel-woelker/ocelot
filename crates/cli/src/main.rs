use ocelot_base::cli::format_cli_error;
use ocelot_base::result::OcelotResult;
use ocelot_engine::engine::Engine;
use ocelot_engine::failed_test_result::FailedTestResult;
use ocelot_engine::test_run_summary::TestRunSummary;
use ocelot_pal::pal::PalHandle;
use ocelot_pal::pal_real::PalReal;
use pico_args::Arguments;
use std::process::ExitCode;

#[derive(Debug, PartialEq, Eq)]
enum CliCommand {
    Run { script_path: String },
    Test { script_path: String },
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
    let mut args = Arguments::from_vec(pal.args());
    let Some(command) = parse_command(&mut args) else {
        print_usage();
        return ExitCode::FAILURE;
    };

    if !args.finish().is_empty() {
        print_usage();
        return ExitCode::FAILURE;
    }

    execute_command(pal, &command)
}

fn parse_command(args: &mut Arguments) -> Option<CliCommand> {
    let first_arg = args.subcommand().ok().flatten()?;

    match first_arg.as_str() {
        "test" => Some(CliCommand::Test {
            script_path: args.free_from_str().ok()?,
        }),
        script_path => Some(CliCommand::Run {
            script_path: script_path.to_owned(),
        }),
    }
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
    let engine = Engine::new(pal);

    match command {
        CliCommand::Run { script_path } => {
            engine.run_script(script_path.as_str())?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Test { script_path } => {
            let summary = engine.run_tests(script_path.as_str())?;
            report_test_summary(&summary);
            Ok(exit_code_for_test_summary(&summary))
        }
    }
}

fn exit_code_for_test_summary(summary: &TestRunSummary) -> ExitCode {
    if summary.is_success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_usage() {
    eprintln!("Usage:\n  ocelot <script-file>\n  ocelot test <script-file>");
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
    use super::{CliCommand, parse_command, render_test_summary_lines, run_command};
    use ocelot_engine::failed_test_result::FailedTestResult;
    use ocelot_engine::test_run_summary::TestRunSummary;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;
    use pico_args::Arguments;
    use std::process::ExitCode;

    #[test]
    fn parses_run_command() {
        let mut args = Arguments::from_vec(vec!["examples/hello.ocelot".into()]);

        assert_eq!(
            parse_command(&mut args),
            Some(CliCommand::Run {
                script_path: "examples/hello.ocelot".into(),
            })
        );
    }

    #[test]
    fn parses_test_command() {
        let mut args = Arguments::from_vec(vec!["test".into(), "examples/tests.ocelot".into()]);

        assert_eq!(
            parse_command(&mut args),
            Some(CliCommand::Test {
                script_path: "examples/tests.ocelot".into(),
            })
        );
    }

    #[test]
    fn rejects_missing_script_path_for_test_command() {
        let mut args = Arguments::from_vec(vec!["test".into()]);

        assert_eq!(parse_command(&mut args), None);
    }

    #[test]
    fn parses_plain_script_execution() {
        let pal = PalMock::new();
        pal.set_file("examples/hello.ocelot", "println(\"hello\");");
        pal.set_args(["examples/hello.ocelot"]);

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
    fn parses_arguments_from_pal() {
        let pal = PalMock::new();
        pal.set_args(["examples/hello.ocelot"]);
        pal.set_file("examples/hello.ocelot", "println(\"hello\");");

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
