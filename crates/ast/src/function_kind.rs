use crate::function_item::FunctionItem;
use crate::native_function::NativeFunction;
use ocelot_base::source_file::SourceFile;

/// Runtime-facing classification of one function definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionKind {
    Native {
        native_function: NativeFunction,
    },
    UserDefined {
        function: Box<FunctionItem>,
        source_file: Box<SourceFile>,
    },
}
