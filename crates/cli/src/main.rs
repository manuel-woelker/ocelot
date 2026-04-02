mod cli_command;

use crate::cli_command::CliCommand;
use ocelot_base::cli::try_main;
use ocelot_base::result::OcelotResult;
use ocelot_engine::engine::Engine;
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
            for test_name in summary.passed {
                println!("PASS {test_name}");
            }
            Ok(())
        }
    }
}

fn parse_command() -> Option<CliCommand> {
    parse_command_inner(std::env::args().skip(1).collect())
}

fn print_usage() {
    eprintln!("Usage:\n  ocelot <script-file>\n  ocelot test <script-file>");
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
    use super::parse_command_inner;
    use crate::cli_command::CliCommand;

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
}
