/// Execution engine used for one complete spec pass.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ValidationBackend {
    /// Existing tree-walking execution.
    Interpreter,
    /// Real C/TCC build and standalone process execution.
    CTcc,
}
