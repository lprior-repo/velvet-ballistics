#![forbid(unsafe_code)]
//! Monotonic per-run event sequence.

/// Monotonic per-run event sequence.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[repr(transparent)]
pub struct EventSeq(u64);

impl EventSeq {
    /// Creates an event sequence.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw sequence value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Zero event sequence.
    pub const ZERO: Self = Self(0);
    /// Minimum event sequence.
    pub const MIN: Self = Self(0);
    /// Maximum event sequence.
    pub const MAX: Self = Self(u64::MAX);
}

#[cfg(kani)]
impl kani::Arbitrary for EventSeq {
    fn any() -> Self {
        Self::new(kani::any())
    }
}
