use ocelot_base::cli::try_main_with_headline;
use ocelot_base::file_path::FilePath;
use ocelot_spec_validation::capturing_pal::CapturingPal;
use ocelot_spec_validation::render_validation_report::render_validation_report;
use ocelot_spec_validation::validate_spec_directory::validate_spec_directory_with_backend;
use ocelot_spec_validation::validation_backend::ValidationBackend;
use std::process::ExitCode;

fn main() -> ExitCode {
    try_main_with_headline("spec validation failed", || {
        let pal = CapturingPal::new(ocelot_pal::pal_real::PalReal::new_handle()?);
        let mode = std::env::args().skip(1).collect::<Vec<_>>();
        let backends = match mode.as_slice() {
            [] => vec![ValidationBackend::Interpreter],
            [arg] if arg == "--backend=interpreter" => vec![ValidationBackend::Interpreter],
            [arg] if arg == "--backend=c-tcc" => vec![ValidationBackend::CTcc],
            [arg] if arg == "--backend=all" => {
                vec![ValidationBackend::Interpreter, ValidationBackend::CTcc]
            }
            _ => ocelot_base::bail!(
                "usage: ocelot-spec-validation [--backend=interpreter|c-tcc|all]"
            ),
        };
        let mut failed = 0;
        for backend in backends {
            let report = validate_spec_directory_with_backend(
                &pal,
                &FilePath::from("docs/spec"),
                &std::env::temp_dir().join("spec-validation"),
                backend,
            )?;
            println!(
                "Backend: {}\n{}",
                if backend == ValidationBackend::CTcc {
                    "c-tcc"
                } else {
                    "interpreter"
                },
                render_validation_report(&report)
            );
            failed += report.failures.len();
        }
        if failed != 0 {
            ocelot_base::bail!("spec validation found {failed} failure(s)");
        }
        Ok(())
    })
}
