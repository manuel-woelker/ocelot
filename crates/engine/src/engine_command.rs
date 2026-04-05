use ocelot_base::file_path::FilePath;
use ocelot_base::result::{OcelotResult, OptionExt};
use ocelot_base::shared_string::SharedString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineCommand {
    pub base_path: FilePath,
    pub kind: RunCommandKind,
}

impl EngineCommand {
    pub fn run_file(path: FilePath) -> OcelotResult<Self> {
        Ok(Self {
            base_path: parent_path(&path)?,
            kind: RunCommandKind::RunFile { path },
        })
    }

    pub fn run_test(path: FilePath, test_name: impl Into<SharedString>) -> OcelotResult<Self> {
        Ok(Self {
            base_path: parent_path(&path)?,
            kind: RunCommandKind::RunTest {
                path,
                test_name: test_name.into(),
            },
        })
    }

    pub fn run_tests(path: FilePath) -> OcelotResult<Self> {
        Ok(Self {
            base_path: parent_path(&path)?,
            kind: RunCommandKind::RunTests { path },
        })
    }

    pub fn entry_path(&self) -> &FilePath {
        match &self.kind {
            RunCommandKind::RunFile { path }
            | RunCommandKind::RunTest { path, .. }
            | RunCommandKind::RunTests { path } => path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunCommandKind {
    RunFile {
        path: FilePath,
    },
    RunTest {
        path: FilePath,
        test_name: SharedString,
    },
    RunTests {
        path: FilePath,
    },
}

fn parent_path(path: &FilePath) -> OcelotResult<FilePath> {
    path.parent()
        .with_context(|| format!("Failed to get parent of {path}"))
}
