use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;

/// Semantic role assigned to one source file based on its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFileKind {
    Module,
    Script,
}

impl SourceFileKind {
    /// Infers a source file kind from a path extension.
    pub fn from_path(path: &FilePath) -> OcelotResult<Self> {
        match path.extension() {
            Some("ocelot") => Ok(Self::Module),
            Some("ocelot-script") => Ok(Self::Script),
            _ => ocelot_base::bail!(
                "unsupported source file `{path}`: expected `.ocelot` or `.ocelot-script`"
            ),
        }
    }

    /// Returns whether this kind allows top-level executable statements.
    pub fn allows_top_level_statements(self) -> bool {
        matches!(self, Self::Script)
    }
}

#[cfg(test)]
mod tests {
    use super::SourceFileKind;
    use ocelot_base::file_path::FilePath;

    #[test]
    fn classifies_module_paths() {
        assert_eq!(
            SourceFileKind::from_path(&FilePath::from("examples/helper.ocelot")).unwrap(),
            SourceFileKind::Module
        );
    }

    #[test]
    fn classifies_script_paths() {
        assert_eq!(
            SourceFileKind::from_path(&FilePath::from("examples/main.ocelot-script")).unwrap(),
            SourceFileKind::Script
        );
    }

    #[test]
    fn rejects_unknown_extensions() {
        let error = SourceFileKind::from_path(&FilePath::from("examples/notes.txt")).unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("expected `.ocelot` or `.ocelot-script`")
        );
    }
}
