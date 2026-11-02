/// Kinds of types currently modeled in the program environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Any,
    Boolean,
    String,
    /// The result of a function that returns no value.
    Unit,
    Unresolved,
}
