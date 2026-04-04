use ocelot_ast::type_kind::TypeKind;

/// One native function implementation signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFunctionSignature {
    pub argument_types: Vec<TypeKind>,
}

impl NativeFunctionSignature {
    /// Creates one native signature from argument types.
    pub fn new(argument_types: Vec<TypeKind>) -> Self {
        Self { argument_types }
    }
}
