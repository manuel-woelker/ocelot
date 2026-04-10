/// The kind of one embedded-engine step declared in a spec example.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecExampleEmbeddedStepKind {
    Execute,
    Eval,
}
