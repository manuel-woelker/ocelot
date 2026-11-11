use crate::captured_process::CapturedProcess;
use crate::native_build_request::NativeBuildRequest;
use crate::native_toolchain::NativeToolchain;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_pal::pal::Pal;
use ocelot_pal::process_command::ProcessCommand;
use std::time::Duration;

/// External TCC compiler and explicit GNU-compatible system linker driver.
pub struct TccToolchain {
    /// Compiler executable, which may be a tool-tool wrapper.
    pub compiler: SharedString,
    /// Fixed wrapper arguments, distinct from user paths and compiler flags.
    pub compiler_arguments: Vec<SharedString>,
    /// Config root required when invoking the repository's tool-tool wrapper.
    pub compiler_directory: Option<FilePath>,
    /// Linker driver, normally cc.
    pub linker: SharedString,
    /// Per-stage deadline.
    pub timeout: Duration,
}

impl TccToolchain {
    /// Uses the repository's pinned tool-tool command.
    pub fn repository() -> Self {
        Self {
            compiler: FilePath::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../t"))
                .into_shared_string(),
            compiler_arguments: vec!["tcc".into()],
            compiler_directory: Some(FilePath::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))),
            linker: "cc".into(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Constructs one subprocess invocation using argument vectors only.
    pub fn command(&self, compile: bool, args: Vec<SharedString>) -> ProcessCommand {
        let mut arguments = if compile {
            self.compiler_arguments.clone()
        } else {
            Vec::new()
        };
        arguments.extend(args);
        ProcessCommand {
            executable: if compile {
                self.compiler.clone()
            } else {
                self.linker.clone()
            },
            arguments,
            working_directory: if compile {
                self.compiler_directory.clone()
            } else {
                None
            },
            environment: Vec::new(),
        }
    }
}

impl NativeToolchain for TccToolchain {
    fn check(&self, pal: &dyn Pal) -> OcelotResult<()> {
        if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            ocelot_base::bail!("native builds currently require x86-64 Linux");
        }
        CapturedProcess::run(pal, &self.command(true, vec!["-v".into()]), self.timeout)?
            .require_success("TCC setup")
    }

    fn build(&self, pal: &dyn Pal, request: &NativeBuildRequest) -> OcelotResult<()> {
        let compile = self.command(
            true,
            vec![
                "-c".into(),
                request.source.as_str().into(),
                "-o".into(),
                request.object.as_str().into(),
            ],
        );
        CapturedProcess::run(pal, &compile, self.timeout)?.require_success("TCC C compilation")?;
        /* 📖 # Why omit GNU ld's unwind lookup header?
        The pinned TCC emits overlapping FDE ranges for the generated cleanup helpers.
        GNU ld rejects --eh-frame-hdr for these objects. Generated functions never
        unwind; retaining .eh_frame without its lookup header links correctly.
        */
        let link = self.command(
            false,
            vec![
                "-Wl,--gc-sections".into(),
                "-Wl,--no-eh-frame-hdr".into(),
                request.object.as_str().into(),
                "-o".into(),
                request.executable.as_str().into(),
            ],
        );
        CapturedProcess::run(pal, &link, self.timeout)?.require_success("native linking")?;
        if !pal.file_exists(&request.executable)? {
            ocelot_base::bail!("native linker did not produce an executable");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::TccToolchain;
    use crate::native_build_request::NativeBuildRequest;
    use crate::native_toolchain::NativeToolchain;
    use ocelot_base::timestamp::Timestamp;
    use ocelot_pal::pal_mock::PalMock;
    use ocelot_pal::process_result::ProcessResult;

    #[test]
    fn compiler_and_linker_failures_stay_tool_failures() {
        let toolchain = TccToolchain::repository();
        let request = NativeBuildRequest {
            source: "/build/source file.c".into(),
            object: "/build/object file.o".into(),
            executable: "/build/output file".into(),
        };
        let pal = PalMock::new();
        let compile = toolchain.command(
            true,
            vec![
                "-c".into(),
                request.source.as_str().into(),
                "-o".into(),
                request.object.as_str().into(),
            ],
        );
        assert_eq!(compile.arguments[2], "/build/source file.c");
        pal.set_process_execution(
            compile.clone(),
            Vec::new(),
            ProcessResult {
                started_at: Timestamp::new(0),
                finished_at: Timestamp::new(1),
                exit_code: Some(1),
            },
        );
        let error = toolchain.build(&pal, &request).unwrap_err();
        assert!(
            error
                .kind()
                .to_string()
                .contains("TCC C compilation failed")
        );

        pal.set_process_execution(
            compile,
            Vec::new(),
            ProcessResult {
                started_at: Timestamp::new(0),
                finished_at: Timestamp::new(1),
                exit_code: Some(0),
            },
        );
        let link = toolchain.command(
            false,
            vec![
                "-Wl,--gc-sections".into(),
                "-Wl,--no-eh-frame-hdr".into(),
                request.object.as_str().into(),
                "-o".into(),
                request.executable.as_str().into(),
            ],
        );
        pal.set_process_execution(
            link,
            Vec::new(),
            ProcessResult {
                started_at: Timestamp::new(0),
                finished_at: Timestamp::new(1),
                exit_code: Some(1),
            },
        );
        assert!(
            toolchain
                .build(&pal, &request)
                .unwrap_err()
                .kind()
                .to_string()
                .contains("native linking failed")
        );
    }
}
