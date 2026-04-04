use crate::function_kind::FunctionKind;
use crate::type_index::TypeIndex;
use ocelot_base::shared_string::SharedString;

/// Definition record for one function entry in the program environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    pub name: SharedString,
    pub argument_types: Vec<TypeIndex>,
    pub kind: FunctionKind,
}

impl FunctionDefinition {
    /// Creates a new native function definition.
    pub fn native(
        name: impl Into<SharedString>,
        argument_types: Vec<TypeIndex>,
        native_function: crate::native_function::NativeFunction,
    ) -> Self {
        Self {
            name: name.into(),
            argument_types,
            kind: FunctionKind::Native { native_function },
        }
    }

    /// Creates a new user-defined function definition.
    pub fn user_defined(function: crate::function_item::FunctionItem) -> Self {
        Self {
            name: function.identifier.name.clone(),
            argument_types: Vec::new(),
            kind: FunctionKind::UserDefined {
                function: Box::new(function),
            },
        }
    }
}
