use ocelot_base::RwLock;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::timestamp::Timestamp;
use ocelot_pal::pal::{FileChangeCallback, Pal, PalHandle, ReadSeek};
use ocelot_pal::process_command::ProcessCommand;
use ocelot_pal::process_event_sink::ProcessEventSink;
use ocelot_pal::process_result::ProcessResult;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// What PAL implementation captures printed output while delegating everything else?
#[derive(Debug, Clone)]
pub struct CapturingPal {
    inner: PalHandle,
    printed_output: Arc<RwLock<String>>,
}

impl CapturingPal {
    /// Creates a capturing PAL around another PAL handle.
    pub fn new(inner: PalHandle) -> Self {
        Self {
            inner,
            printed_output: Arc::new(RwLock::new(String::new())),
        }
    }

    /// Returns the captured printed output and clears the buffer.
    pub fn take_printed_output(&self) -> String {
        std::mem::take(&mut *self.printed_output.write())
    }
}

impl Pal for CapturingPal {
    fn file_exists(&self, path: &FilePath) -> OcelotResult<bool> {
        self.inner.file_exists(path)
    }

    fn read_file(&self, path: &FilePath) -> OcelotResult<Box<dyn ReadSeek + 'static>> {
        self.inner.read_file(path)
    }

    fn read_file_to_string(
        &self,
        path: &FilePath,
    ) -> OcelotResult<ocelot_base::shared_string::SharedString> {
        self.inner.read_file_to_string(path)
    }

    fn read_file_to_end(&self, path: &FilePath) -> OcelotResult<Vec<u8>> {
        self.inner.read_file_to_end(path)
    }

    fn walk_directory(
        &self,
        path: &FilePath,
        globs: &[String],
    ) -> OcelotResult<Box<dyn Iterator<Item = OcelotResult<FilePath>> + '_>> {
        self.inner.walk_directory(path, globs)
    }

    fn watch_directory(
        &self,
        directory: &FilePath,
        globs: &[String],
        callback: FileChangeCallback,
    ) -> OcelotResult<()> {
        self.inner.watch_directory(directory, globs, callback)
    }

    fn create_directory_all(&self, path: &FilePath) -> OcelotResult<()> {
        self.inner.create_directory_all(path)
    }

    fn create_directory(&self, path: &FilePath) -> OcelotResult<bool> {
        self.inner.create_directory(path)
    }

    fn write_file(&self, path: &FilePath, content: &[u8]) -> OcelotResult<()> {
        self.inner.write_file(path, content)
    }

    fn append_file(&self, path: &FilePath, content: &[u8]) -> OcelotResult<()> {
        self.inner.append_file(path, content)
    }

    fn print(&self, text: &str) -> OcelotResult<()> {
        self.printed_output.write().push_str(text);
        Ok(())
    }

    fn is_interactive_terminal(&self) -> bool {
        self.inner.is_interactive_terminal()
    }

    fn default_parallelism(&self) -> usize {
        self.inner.default_parallelism()
    }

    fn run_process(
        &self,
        command: &ProcessCommand,
        sink: &mut dyn ProcessEventSink,
    ) -> OcelotResult<ProcessResult> {
        self.inner.run_process(command, sink)
    }

    fn now(&self) -> Timestamp {
        self.inner.now()
    }

    fn system_time(&self) -> SystemTime {
        self.inner.system_time()
    }

    fn sleep(&self, duration: Duration) {
        self.inner.sleep(duration);
    }
}
