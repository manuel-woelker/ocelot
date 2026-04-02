use ocelot_base::cli::try_main_with_headline;
use ocelot_base::file_path::FilePath;
use ocelot_spec_validation::capturing_pal::CapturingPal;
use ocelot_spec_validation::run_validation_runner::run_validation_runner;
use std::process::ExitCode;

fn main() -> ExitCode {
    try_main_with_headline("spec validation failed", || {
        let pal = CapturingPal::new(ocelot_pal::pal_real::PalReal::new_handle()?);
        run_validation_runner(
            &pal,
            &FilePath::from("docs/spec"),
            &std::env::temp_dir().join("ocelot-spec-validation"),
        )
    })
}
