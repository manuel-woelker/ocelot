use crate::function_kind::FunctionKind;
use ocelot_base::shared_string::SharedString;

/// Definition record for one function entry in the program environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    pub name: SharedString,
    pub kind: FunctionKind,
}

impl FunctionDefinition {
    /// Creates a new native function definition.
    pub fn native(
        name: impl Into<SharedString>,
        native_function: crate::native_function::NativeFunction,
    ) -> Self {
        Self {
            name: name.into(),
            kind: FunctionKind::Native { native_function },
        }
    }

    /// Creates a new user-defined function definition.
    pub fn user_defined(function: crate::function_item::FunctionItem) -> Self {
        Self {
            name: function.name.clone(),
            kind: FunctionKind::UserDefined {
                function: Box::new(function),
            },
        }
    }
}
