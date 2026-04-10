/// Kinds of validation failures reported by the spec validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationFailureKind {
    /// The markdown example shape did not satisfy the extraction contract.
    MalformedExample,
    /// The observed output did not match the expected output block.
    OutputMismatch,
    /// The observed eval result did not match the expected eval result block.
    EvalResultMismatch,
    /// The observed error text did not match the expected error block.
    ErrorMismatch,
    /// Execution succeeded when the example expected an error.
    ExpectedErrorButSucceeded,
    /// Execution failed when the example expected output.
    ExpectedOutputButFailed,
    /// Execution failed when the example expected an eval result.
    ExpectedEvalResultButFailed,
}
