use ocelot_base::result::OcelotResult;
use ocelot_pal::pal::Pal;
use ocelot_pal::process_command::ProcessCommand;
use ocelot_pal::process_event::ProcessEvent;
use ocelot_pal::process_event_sink::ProcessEventSink;
use ocelot_pal::process_output_stream::ProcessOutputStream;
use std::time::Duration;

/// Raw subprocess outcome; infrastructure errors remain distinct from language errors.
#[derive(Default)]
pub struct CapturedProcess {
    /// Bytes emitted on stdout.
    pub stdout: Vec<u8>,
    /// Bytes emitted on stderr.
    pub stderr: Vec<u8>,
    /// Exit code, absent when terminated by a signal.
    pub exit_code: Option<i32>,
}

impl ProcessEventSink for CapturedProcess {
    fn handle_event(&mut self, event: ProcessEvent) -> OcelotResult<()> {
        if let ProcessEvent::Output(event) = event {
            match event.stream {
                ProcessOutputStream::Stdout => self.stdout.extend(event.bytes),
                ProcessOutputStream::Stderr => self.stderr.extend(event.bytes),
            }
        }
        Ok(())
    }
}

impl CapturedProcess {
    /// Captures both streams while enforcing the PAL's process deadline.
    pub fn run(pal: &dyn Pal, command: &ProcessCommand, timeout: Duration) -> OcelotResult<Self> {
        let mut capture = Self::default();
        capture.exit_code = pal
            .run_process_with_timeout(command, &mut capture, timeout)?
            .exit_code;
        Ok(capture)
    }

    /// Checks a compiler/linker command, preserving raw diagnostics on failure.
    pub fn require_success(&self, stage: &str) -> OcelotResult<()> {
        if self.exit_code != Some(0) {
            ocelot_base::bail!(
                "{stage} failed ({:?}):\n{}\n{}",
                self.exit_code,
                String::from_utf8_lossy(&self.stdout),
                String::from_utf8_lossy(&self.stderr)
            );
        }
        Ok(())
    }
}
