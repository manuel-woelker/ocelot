use crate::source_file_kind::SourceFileKind;
use ocelot_ast::script::Script;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;

/// One parsed module participating in a multi-file program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedModule {
    pub module_name: SharedString,
    pub kind: SourceFileKind,
    pub source_file: SourceFile,
    pub script: Script,
}

impl LoadedModule {
    /// Creates a loaded module from its logical name, source file, and parsed script.
    pub fn new(
        module_name: impl Into<SharedString>,
        kind: SourceFileKind,
        source_file: SourceFile,
        script: Script,
    ) -> Self {
        Self {
            module_name: module_name.into(),
            kind,
            source_file,
            script,
        }
    }
}
