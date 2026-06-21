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
mod tests;
