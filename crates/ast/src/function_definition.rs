use crate::function_kind::FunctionKind;
use crate::type_index::TypeIndex;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;

/// Definition record for one function entry in the program environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    pub name: SharedString,
    pub module_name: SharedString,
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
            module_name: SharedString::empty(),
            argument_types,
            kind: FunctionKind::Native { native_function },
        }
    }

    /// Creates a new user-defined function definition.
    pub fn user_defined(
        module_name: impl Into<SharedString>,
        qualified_name: impl Into<SharedString>,
        function: crate::function_item::FunctionItem,
        source_file: SourceFile,
    ) -> Self {
        Self {
            name: qualified_name.into(),
            module_name: module_name.into(),
            argument_types: Vec::new(),
            kind: FunctionKind::UserDefined {
                function: Box::new(function),
                source_file: Box::new(source_file),
            },
        }
    }
}
