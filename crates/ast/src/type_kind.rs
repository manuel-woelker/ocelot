/// Kinds of types currently modeled in the program environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Boolean,
    String,
    Unresolved,
}
