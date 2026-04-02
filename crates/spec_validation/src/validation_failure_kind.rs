/// What kind of validation failure occurred?
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationFailureKind {
    /// The markdown example shape did not satisfy the extraction contract.
    MalformedExample,
    /// The observed output did not match the expected output block.
    OutputMismatch,
}
