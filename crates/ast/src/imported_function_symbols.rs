use crate::function_index::FunctionIndex;
use ocelot_base::file_path::FilePath;
use ocelot_base::shared_string::SharedString;
use std::collections::HashMap;

/// File-local imported function bindings keyed by source file and local name.
pub type ImportedFunctionSymbols = HashMap<FilePath, HashMap<SharedString, FunctionIndex>>;
