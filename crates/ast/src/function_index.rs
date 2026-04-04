use std::num::NonZeroU32;

/// Compact typed handle for one function entry in the program environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FunctionIndex(NonZeroU32);

impl FunctionIndex {
    /// Creates a function index from a one-based table position.
    pub fn new(index: u32) -> Self {
        Self(NonZeroU32::new(index).expect("function index 0 is reserved"))
    }

    /// Returns the table position as `usize`.
    pub fn as_usize(self) -> usize {
        self.0.get() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::FunctionIndex;

    #[test]
    fn function_index_round_trips_a_one_based_position() {
        let index = FunctionIndex::new(2);

        assert_eq!(index.as_usize(), 2);
    }
}
