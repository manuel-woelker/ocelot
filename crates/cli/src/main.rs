use ocelot_base::cli::try_main;
use ocelot_base::result::OcelotResult;
use ocelot_engine::engine::Engine;
use ocelot_pal::pal_real::PalReal;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(script_path) = parse_script_path() else {
        print_usage();
        return ExitCode::FAILURE;
    };

    try_main(|| run_cli(&script_path))
}

fn run_cli(script_path: &str) -> OcelotResult<()> {
    let pal = PalReal::new_handle()?;
    let engine = Engine::new(pal);
    engine.run_script(script_path)
}

fn parse_script_path() -> Option<String> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.as_slice() {
        [script_path] => Some(script_path.clone()),
        _ => None,
    }
}

fn print_usage() {
    eprintln!("Usage:\n  ocelot <script-file>");
}
