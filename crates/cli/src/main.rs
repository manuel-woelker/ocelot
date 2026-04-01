use ocelot_base::cli::try_main;
use ocelot_base::result::OcelotResult;
use std::process::ExitCode;

fn main() -> ExitCode {
    try_main(main_placeholder)
}

fn main_placeholder() -> OcelotResult<()> {
    println!("ocelot CLI placeholder");
    Ok(())
}
