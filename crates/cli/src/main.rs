use ocelot_base::cli::try_main;
use ocelot_base::error::OcelotError;
use ocelot_base::result::OcelotResult;
use ocelot_engine::engine::Engine;
use ocelot_pal::pal_real::PalReal;
use std::process::ExitCode;

fn main() -> ExitCode {
    try_main(run_cli)
}

fn run_cli() -> OcelotResult<()> {
    let script_path = std::env::args()
        .nth(1)
        .ok_or_else(|| OcelotError::message("missing script filepath argument"))?;
    let pal = PalReal::new_handle()?;
    let engine = Engine::new(pal);
    engine.run_script(script_path)
}
