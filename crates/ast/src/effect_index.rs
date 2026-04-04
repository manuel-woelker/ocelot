use std::num::NonZeroU32;

/// Compact typed handle for one effect entry in the program environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EffectIndex(NonZeroU32);

impl EffectIndex {
    /// Creates an effect index from a one-based table position.
    pub fn new(index: u32) -> Self {
        Self(NonZeroU32::new(index).expect("effect index 0 is reserved"))
    }

    /// Returns the table position as `usize`.
    pub fn as_usize(self) -> usize {
        self.0.get() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::EffectIndex;

    #[test]
    fn effect_index_round_trips_a_one_based_position() {
        let index = EffectIndex::new(2);

        assert_eq!(index.as_usize(), 2);
    }
}
