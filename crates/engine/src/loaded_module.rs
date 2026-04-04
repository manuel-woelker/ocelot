use ocelot_ast::script::Script;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;

/// One parsed module participating in a multi-file program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedModule {
    pub module_name: SharedString,
    pub source_file: SourceFile,
    pub script: Script,
}

impl LoadedModule {
    /// Creates a loaded module from its logical name, source file, and parsed script.
    pub fn new(
        module_name: impl Into<SharedString>,
        source_file: SourceFile,
        script: Script,
    ) -> Self {
        Self {
            module_name: module_name.into(),
            source_file,
            script,
        }
    }
}
