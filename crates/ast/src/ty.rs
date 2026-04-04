use crate::type_kind::TypeKind;
use ocelot_base::shared_string::SharedString;

/// Definition record for one type entry in the program environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ty {
    pub name: SharedString,
    pub kind: TypeKind,
}

impl Ty {
    /// Creates one type definition entry.
    pub fn new(name: impl Into<SharedString>, kind: TypeKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }
}
