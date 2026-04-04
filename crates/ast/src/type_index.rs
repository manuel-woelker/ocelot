/// Compact typed handle for one type entry in the program environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TypeIndex(u32);

impl TypeIndex {
    /// Creates a type index from one table position.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns the canonical unresolved type index.
    pub fn unresolved() -> Self {
        Self(0)
    }

    /// Returns whether this index still points at the unresolved sentinel.
    pub fn is_unresolved(self) -> bool {
        self.0 == 0
    }

    /// Returns the table position as `usize`.
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

#[cfg(test)]
mod tests {
    use super::TypeIndex;

    #[test]
    fn unresolved_type_index_uses_slot_zero() {
        assert_eq!(TypeIndex::unresolved().as_usize(), 0);
        assert!(TypeIndex::unresolved().is_unresolved());
    }

    #[test]
    fn type_index_round_trips_a_table_position() {
        let index = TypeIndex::new(2);

        assert_eq!(index.as_usize(), 2);
        assert!(!index.is_unresolved());
    }
}
