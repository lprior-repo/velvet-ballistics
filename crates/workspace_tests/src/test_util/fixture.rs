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
mod tests {
    use super::*;

    #[test]
    fn zero_capacity_rejected() {
        let result = FixtureCapacity::new(0);
        assert!(matches!(result, Err(TestSetupError::InvalidCapacity(_))));
    }

    #[test]
    fn valid_capacity_accepted() {
        let result = FixtureCapacity::new(100);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, 100);
    }

    #[test]
    fn max_capacity_boundary() {
        let result = FixtureCapacity::new(FixtureCapacity::MAX_CAPACITY);
        assert!(result.is_ok());
    }

    #[test]
    fn over_max_capacity_rejected() {
        let result = FixtureCapacity::new(FixtureCapacity::MAX_CAPACITY + 1);
        assert!(matches!(result, Err(TestSetupError::InvalidCapacity(_))));
    }
}
