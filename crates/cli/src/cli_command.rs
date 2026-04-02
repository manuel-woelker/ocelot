/// Supported CLI command shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Run { script_path: String },
    Test { script_path: String },
}
