use crate::source_file_kind::SourceFileKind;
use ocelot_ast::compilation_unit::CompilationUnit;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;

/// One parsed module participating in a multi-file program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModule {
    pub module_name: SharedString,
    pub kind: SourceFileKind,
    pub source_file: SourceFile,
    pub compilation_unit: CompilationUnit,
}

impl ParsedModule {
    /// Creates a loaded module from its logical name, source file, and parsed compilation unit.
    pub fn new(
        module_name: impl Into<SharedString>,
        kind: SourceFileKind,
        source_file: SourceFile,
        compilation_unit: CompilationUnit,
    ) -> Self {
        Self {
            module_name: module_name.into(),
            kind,
            source_file,
            compilation_unit,
        }
    }
}
