use ocelot_base::RwLock;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::timestamp::Timestamp;
use ocelot_pal::pal::{FileChangeCallback, Pal, PalHandle, ReadSeek};
use ocelot_pal::process_command::ProcessCommand;
use ocelot_pal::process_event_sink::ProcessEventSink;
use ocelot_pal::process_result::ProcessResult;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsString;
use std::io::Cursor;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// PAL implementation that captures printed output while delegating other behavior.
#[derive(Debug, Clone)]
pub struct CapturingPal {
    inner: PalHandle,
    printed_output: Arc<RwLock<String>>,
    virtual_files: Arc<RwLock<HashMap<FilePath, Vec<u8>>>>,
    virtual_directories: Arc<RwLock<HashSet<FilePath>>>,
}

impl CapturingPal {
    /// Creates a capturing PAL around another PAL handle.
    pub fn new(inner: PalHandle) -> Self {
        Self {
            inner,
            printed_output: Arc::new(RwLock::new(String::new())),
            virtual_files: Arc::new(RwLock::new(HashMap::new())),
            virtual_directories: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Returns the captured printed output and clears the buffer.
    pub fn take_printed_output(&self) -> String {
        std::mem::take(&mut *self.printed_output.write())
    }

    /// Clears all virtual files and directories created for one example run.
    pub fn clear_virtual_files(&self) {
        self.virtual_files.write().clear();
        self.virtual_directories.write().clear();
    }
}

impl Pal for CapturingPal {
    fn args(&self) -> Vec<OsString> {
        self.inner.args()
    }

    fn file_exists(&self, path: &FilePath) -> OcelotResult<bool> {
        if self.virtual_files.read().contains_key(path) {
            return Ok(true);
        }
        self.inner.file_exists(path)
    }

    fn read_file(&self, path: &FilePath) -> OcelotResult<Box<dyn ReadSeek + 'static>> {
        if let Some(content) = self.virtual_files.read().get(path).cloned() {
            return Ok(Box::new(Cursor::new(content)));
        }
        self.inner.read_file(path)
    }

    fn read_file_to_string(
        &self,
        path: &FilePath,
    ) -> OcelotResult<ocelot_base::shared_string::SharedString> {
        if let Some(content) = self.virtual_files.read().get(path).cloned() {
            return ocelot_base::shared_string::SharedString::from_utf8(&content);
        }
        self.inner.read_file_to_string(path)
    }

    fn read_file_to_end(&self, path: &FilePath) -> OcelotResult<Vec<u8>> {
        if let Some(content) = self.virtual_files.read().get(path).cloned() {
            return Ok(content);
        }
        self.inner.read_file_to_end(path)
    }

    fn walk_directory(
        &self,
        path: &FilePath,
        globs: &[String],
    ) -> OcelotResult<Box<dyn Iterator<Item = OcelotResult<FilePath>> + '_>> {
        let mut results = self
            .inner
            .walk_directory(path, globs)
            .map(|entries| entries.collect::<Vec<OcelotResult<FilePath>>>())
            .unwrap_or_default();
        results.extend(
            self.virtual_files
                .read()
                .keys()
                .filter(|file_path| file_path.as_path().starts_with(path.as_path()))
                .cloned()
                .map(Ok),
        );
        Ok(Box::new(results.into_iter()))
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
        self.virtual_directories.write().insert(path.clone());
        Ok(())
    }

    fn create_directory(&self, path: &FilePath) -> OcelotResult<bool> {
        Ok(self.virtual_directories.write().insert(path.clone()))
    }

    fn write_file(&self, path: &FilePath, content: &[u8]) -> OcelotResult<()> {
        self.virtual_files
            .write()
            .insert(path.clone(), content.to_vec());
        Ok(())
    }

    fn append_file(&self, path: &FilePath, content: &[u8]) -> OcelotResult<()> {
        self.virtual_files
            .write()
            .entry(path.clone())
            .and_modify(|existing| existing.extend_from_slice(content))
            .or_insert_with(|| content.to_vec());
        Ok(())
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
