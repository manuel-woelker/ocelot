use ocelot_ast::function_item::FunctionItem;
use ocelot_base::source_file::SourceFile;

use crate::native_function::NativeFunction;

/// Runtime-facing classification of one function definition.
#[derive(Debug, Clone)]
pub enum FunctionKind {
    NativeFunction {
        native_function: Box<dyn NativeFunction>,
    },
    UserDefined {
        function: Box<FunctionItem>,
        source_file: Box<SourceFile>,
    },
}
