use ocelot_base::file_path::FilePath;
use ocelot_base::result::{OcelotResult, OptionExt};

pub struct EngineCommand {
    pub base_path: FilePath,
}

impl EngineCommand {
    pub fn run_file(path: FilePath) -> OcelotResult<Self> {
        Ok(Self { base_path: path.parent().with_context(|| format!("Failed to get parent of {path}"))? })
    }

    pub fn run_test(path: FilePath) -> OcelotResult<Self> {
        Ok(Self { base_path: path.parent().with_context(|| format!("Failed to get parent of {path}"))? })
    }
}

pub enum RunCommandKind {
    Execute {
        path: FilePath,
    },
    Test,
}