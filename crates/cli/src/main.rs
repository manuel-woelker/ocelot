use ocelot_base::cli::format_cli_error;
use ocelot_base::result::OcelotResult;
use ocelot_engine::engine::Engine;
use ocelot_engine::failed_test_result::FailedTestResult;
use ocelot_engine::test_run_summary::TestRunSummary;
use ocelot_pal::pal::Pal;
use ocelot_pal::pal::PalHandle;
use ocelot_pal::pal_real::PalReal;
use pico_args::Arguments;
use std::process::ExitCode;

fn main() -> ExitCode {
    let pal = match PalReal::new_handle() {
        Ok(pal) => pal,
        Err(error) => {
            eprint!("{}", format_cli_error("operation failed", &error));
            return ExitCode::FAILURE;
        }
    };

    run_command(&*pal, pal.clone())
}

fn run_cli(pal: PalHandle, script_path: &str) -> ExitCode {
    match run_script(pal, script_path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprint!("{}", format_cli_error("operation failed", &error));
            ExitCode::FAILURE
        }
    }
}

fn run_script(pal: PalHandle, script_path: &str) -> OcelotResult<()> {
    Engine::new(pal).run_script(script_path)
}

fn run_test_cli(pal: PalHandle, script_path: &str) -> ExitCode {
    match run_tests(pal, script_path) {
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

fn run_tests(pal: PalHandle, script_path: &str) -> OcelotResult<TestRunSummary> {
    Engine::new(pal).run_tests(script_path)
}

fn run_command(args_pal: &dyn Pal, execution_pal: PalHandle) -> ExitCode {
    let mut args = Arguments::from_vec(args_pal.args());
    let Some(subcommand) = args.subcommand().ok().flatten() else {
        print_usage();
        return ExitCode::FAILURE;
    };

    let exit_code = match subcommand.as_str() {
        "test" => {
            let Some(script_path): Option<String> = args.free_from_str().ok() else {
                print_usage();
                return ExitCode::FAILURE;
            };
            run_test_cli(execution_pal, script_path.as_str())
        }
        _ => run_cli(execution_pal, subcommand.as_str()),
    };

    if args.finish().is_empty() {
        exit_code
    } else {
        print_usage();
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
    use super::{render_test_summary_lines, run_command};
    use ocelot_engine::failed_test_result::FailedTestResult;
    use ocelot_engine::test_run_summary::TestRunSummary;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;
    use std::process::ExitCode;

    #[test]
    fn parses_plain_script_execution() {
        let pal = PalMock::new();
        pal.set_file("examples/hello.ocelot", "println(\"hello\");");
        pal.set_args(["examples/hello.ocelot"]);

        assert_eq!(
            run_command(&pal, PalHandle::new(pal.clone())),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn parses_test_execution() {
        let pal = PalMock::new();
        pal.set_file(
            "examples/tests.ocelot",
            "test \"passes\" { println(\"ok\"); }",
        );
        pal.set_args(["test", "examples/tests.ocelot"]);

        assert_eq!(
            run_command(&pal, PalHandle::new(pal.clone())),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn parses_arguments_from_pal() {
        let pal = PalMock::new();
        pal.set_args(["examples/hello.ocelot"]);
        pal.set_file("examples/hello.ocelot", "println(\"hello\");");

        assert_eq!(
            run_command(&pal, PalHandle::new(pal.clone())),
            ExitCode::SUCCESS
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
