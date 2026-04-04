use std::num::NonZeroU32;

/// Compact typed handle for one function entry in the program environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionIndex(NonZeroU32);

impl FunctionIndex {
    /// Creates a function index from a zero-based table position.
    pub fn new(index: u32) -> Self {
        Self(NonZeroU32::new(index + 1).expect("function index overflow"))
    }

    /// Returns the zero-based table position as `usize`.
    pub fn as_usize(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::FunctionIndex;

    #[test]
    fn function_index_round_trips_a_zero_based_position() {
        let index = FunctionIndex::new(2);

        assert_eq!(index.as_usize(), 2);
    }
}
