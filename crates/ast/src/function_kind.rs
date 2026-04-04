use crate::native_function::NativeFunction;

/// Runtime-facing classification of one function definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionKind {
    Native { native_function: NativeFunction },
    UserDefined { item_index: usize },
}
