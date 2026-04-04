use crate::native_function::NativeFunction;
use ocelot_base::shared_string::SharedString;

/// Definition record for one function entry in the program environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    pub name: SharedString,
    pub native_function: NativeFunction,
}

impl FunctionDefinition {
    /// Creates a new function definition.
    pub fn new(name: impl Into<SharedString>, native_function: NativeFunction) -> Self {
        Self {
            name: name.into(),
            native_function,
        }
    }
}
