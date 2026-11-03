use ocelot_ast::type_kind::TypeKind;

/// One native function implementation signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFunctionSignature {
    pub argument_types: Vec<TypeKind>,
    /// Runtime result kind promised by the implementation.
    pub return_type: TypeKind,
}

impl NativeFunctionSignature {
    /// Creates one native signature from argument and result types.
    pub fn new(argument_types: Vec<TypeKind>, return_type: TypeKind) -> Self {
        Self {
            argument_types,
            return_type,
        }
    }
}
