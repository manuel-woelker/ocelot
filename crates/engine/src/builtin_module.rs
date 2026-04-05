use ocelot_base::file_path::FilePath;
use ocelot_base::shared_string::SharedString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinModule {
    pub module_name: SharedString,
    pub source: SharedString,
}

impl BuiltinModule {
    pub fn new(module_name: impl Into<SharedString>, source: impl Into<SharedString>) -> Self {
        Self {
            module_name: module_name.into(),
            source: source.into(),
        }
    }

    pub fn source_file_path(&self) -> FilePath {
        FilePath::from(format!("<builtin:{}>", self.module_name))
    }
}
