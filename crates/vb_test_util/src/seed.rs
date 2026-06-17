#![allow(clippy::panic, clippy::panic_in_result_fn)]

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// A fixed-size byte array generated deterministically from a u64 seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeededBytes<const N: usize> {
    pub bytes: [u8; N],
}

impl<const N: usize> SeededBytes<N> {
    /// Generate `N` deterministic bytes from a `u64` seed.
    ///
    /// # Errors
    ///
    /// Returns `None` only if `N` is zero (empty array). For all other `N`
    /// the operation is infallible.
    pub fn new(seed: u64) -> Option<Self> {
        if N == 0 {
            return None;
        }
        let mut rng = StdRng::seed_from_u64(seed);
        let mut bytes = [0u8; N];
        rng.fill(&mut bytes[..]);
        Some(Self { bytes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_bytes_determinism() {
        let a = match SeededBytes::<32>::new(42) {
            Some(v) => v,
            None => panic!("SeededBytes::<32>::new(42) must succeed for non-zero N"),
        };
        let b = match SeededBytes::<32>::new(42) {
            Some(v) => v,
            None => panic!("SeededBytes::<32>::new(42) must succeed for non-zero N"),
        };
        assert_eq!(a.bytes, b.bytes);
    }

    #[test]
    fn seeded_bytes_different_seeds() {
        let a = match SeededBytes::<32>::new(42) {
            Some(v) => v,
            None => panic!("SeededBytes::<32>::new(42) must succeed for non-zero N"),
        };
        let b = match SeededBytes::<32>::new(43) {
            Some(v) => v,
            None => panic!("SeededBytes::<32>::new(43) must succeed for non-zero N"),
        };
        assert_ne!(a.bytes, b.bytes);
    }

    #[test]
    fn seeded_bytes_zero_capacity() {
        let result = SeededBytes::<0>::new(42);
        assert!(result.is_none());
    }

    #[test]
    fn seeded_bytes_single_byte() {
        let a = match SeededBytes::<1>::new(0) {
            Some(v) => v,
            None => panic!("SeededBytes::<1>::new(0) must succeed"),
        };
        let b = match SeededBytes::<1>::new(0) {
            Some(v) => v,
            None => panic!("SeededBytes::<1>::new(0) must succeed"),
        };
        assert_eq!(a.bytes, b.bytes);
        assert_eq!(a.bytes.len(), 1);
    }
}
