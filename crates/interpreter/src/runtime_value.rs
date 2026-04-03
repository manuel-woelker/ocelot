use ocelot_base::shared_string::SharedString;

/// Runtime value used by the tree-walking interpreter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValue {
    String(SharedString),
    Unit,
}
