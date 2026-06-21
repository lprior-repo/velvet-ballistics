#![allow(clippy::panic, clippy::panic_in_result_fn)]

use crate::TestSetupError;

/// A capacity bound for fixture builders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixtureCapacity {
    pub value: usize,
}

impl FixtureCapacity {
    /// The maximum allowed fixture capacity.
    pub const MAX_CAPACITY: usize = 1024 * 1024; // 1 MiB

    /// Create a new `FixtureCapacity`.
    ///
    /// # Errors
    ///
    /// Returns `TestSetupError::InvalidCapacity` if `cap` is zero or exceeds
    /// `MAX_CAPACITY`.
    pub fn new(cap: usize) -> Result<Self, TestSetupError> {
        if cap == 0 {
            return Err(TestSetupError::InvalidCapacity(
                "capacity must be greater than zero".to_string(),
            ));
        }
        if cap > Self::MAX_CAPACITY {
            return Err(TestSetupError::InvalidCapacity(format!(
                "capacity {} exceeds maximum {}",
                cap,
                Self::MAX_CAPACITY
            )));
        }
        Ok(Self { value: cap })
    }
}

/// A builder for test fixtures with deterministic seeded contents.
pub struct FixtureBuilder {
    capacity: FixtureCapacity,
}

impl FixtureBuilder {
    /// Create a new `FixtureBuilder` with the given capacity.
    ///
    /// # Errors
    ///
    /// Returns `TestSetupError::InvalidCapacity` if the capacity is invalid.
    pub fn with_capacity(cap: FixtureCapacity) -> Result<Self, TestSetupError> {
        Ok(Self { capacity: cap })
    }

    /// Build a byte vector of the configured capacity using the given seed.
    pub fn build_bytes(self, seed: u64) -> Vec<u8> {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        let mut rng = StdRng::seed_from_u64(seed);
        let mut vec = vec![0u8; self.capacity.value];
        rng.fill(&mut vec[..]);
        vec
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests;
