use crate::capturing_pal::CapturingPal;
use crate::execute_spec_example::{
    main_example_path, normalize_example_text, validate_example_formatting,
};
use crate::expected_outcome::ExpectedOutcome;
use crate::normalize_validation_text::normalize_validation_text;
use crate::observed_outcome::ObservedOutcome;
use crate::render_validation_error::render_validation_error;
use crate::spec_example::SpecExample;
use crate::spec_example_embedded_step_kind::SpecExampleEmbeddedStepKind;
use ocelot_base::error::ErrorKind;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::{OcelotResult, ResultExt};
use ocelot_engine::build_native::build_native;
use ocelot_engine::captured_process::CapturedProcess;
use ocelot_engine::native_scenario_step::NativeScenarioStep;
use ocelot_engine::native_source::{file_source, scenario_source};
use ocelot_engine::tcc_toolchain::TccToolchain;
use ocelot_pal::process_command::ProcessCommand;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Builds and executes one example, with independent sources and typed reporting.
pub fn execute_native_example(
    capture: &CapturingPal,
    execution_root: &std::path::Path,
    example: &SpecExample,
) -> OcelotResult<ObservedOutcome> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    validate_example_formatting(example)?;
    let pal = capture.underlying();
    let root = FilePath::new(execution_root.join(format!(
        "native-{}-{}-{}",
        std::process::id(),
        pal.system_time().duration_since(std::time::UNIX_EPOCH)?.as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    if !pal.create_directory(&root)? {
        ocelot_base::bail!("native workspace collision: {root}");
    }
    let result = (|| {
        for file in &example.source_files {
            let path = root.join(file.path.as_path());
            pal.create_directory_all(&path.parent().unwrap_or_default())?;
            pal.write_file(&path, file.source.as_bytes())?;
        }
        let normalize =
            |text: &str| normalize_validation_text(&normalize_example_text(text, root.as_path()));
        let embedded = !example.embedded_steps.is_empty();
        let unit = if embedded {
            let steps = example
                .embedded_steps
                .iter()
                .map(|s| match s.kind {
                    SpecExampleEmbeddedStepKind::Execute => {
                        NativeScenarioStep::Execute(s.source.clone())
                    }
                    SpecExampleEmbeddedStepKind::Eval => NativeScenarioStep::Eval(s.source.clone()),
                })
                .collect::<Vec<_>>();
            scenario_source(pal.clone(), &root, &steps)?
        } else {
            let path = main_example_path(root.as_path(), example)?;
            match file_source(pal.clone(), &path) {
                Ok(unit) => unit,
                Err(error)
                    if matches!(error.kind(), ErrorKind::CompilationError(_))
                        || error
                            .kind()
                            .to_string()
                            .contains("does not define a `main()` entrypoint") =>
                {
                    return Ok(ObservedOutcome::Error(
                        normalize(&render_validation_error(&error)).into(),
                    ));
                }
                Err(error) => return Err(error),
            }
        };
        if unit.step_count == 0
            && let Some(error) = &unit.deferred_error
        {
            return Ok(ObservedOutcome::Error(
                normalize(&render_validation_error(error)).into(),
            ));
        }
        let executable = root.join("example");
        let report = root.join("result.txt");
        build_native(
            &*pal,
            &unit.source,
            &executable,
            &TccToolchain::repository(),
        )?;
        let run = CapturedProcess::run(
            &*pal,
            &ProcessCommand {
                executable: executable.as_str().into(),
                arguments: vec!["--ocelot-report".into(), report.as_str().into()],
                working_directory: Some(root.clone()),
                environment: Vec::new(),
            },
            Duration::from_secs(5),
        )?;
        let protocol = pal
            .read_file_to_string(&report)
            .context("native execution produced no result protocol")?;
        if run.exit_code == Some(101)
            && let Some(error) = protocol.strip_prefix("ocelot-error-v1\n")
        {
            return Ok(ObservedOutcome::Error(normalize(error).into()));
        }
        run.require_success("native execution")?;
        if !protocol.starts_with("ocelot-eval-v1\n") && protocol.as_str() != "ocelot-ok-v1\n" {
            ocelot_base::bail!("invalid native success protocol");
        }
        if let Some(error) = &unit.deferred_error {
            return Ok(ObservedOutcome::Error(
                normalize(&render_validation_error(error)).into(),
            ));
        }
        if embedded {
            let result = protocol.strip_prefix("ocelot-eval-v1\n").ok_or_else(|| {
                ocelot_base::error::OcelotError::message(
                    "native scenario missing typed eval report",
                )
            })?;
            Ok(ObservedOutcome::EvalResult(normalize(result).into()))
        } else {
            if protocol.as_str() != "ocelot-ok-v1\n" {
                ocelot_base::bail!("invalid native success report");
            }
            Ok(ObservedOutcome::Output(
                normalize(&String::from_utf8(run.stdout)?).into(),
            ))
        }
    })();
    if let Ok(observed) = &result {
        let matches = match (&example.expected_outcome, observed) {
            (ExpectedOutcome::Output(a), ObservedOutcome::Output(b))
            | (ExpectedOutcome::Error(a), ObservedOutcome::Error(b))
            | (ExpectedOutcome::EvalResult(a), ObservedOutcome::EvalResult(b)) => a == b,
            _ => false,
        };
        if matches {
            pal.remove_directory_all(&root)?;
        }
    }
    result.with_context(|| format!("C/TCC example artifacts: {root}"))
}
