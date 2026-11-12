#![forbid(unsafe_code)]

//! Deterministic time source for runtime scheduling.
//!
//! Replaces direct use of `std::time::SystemTime::now()` in runtime primitives
//! so that scheduled operations (e.g., collect primitive elapsed-time
//! computation) can be tested with deterministic, injected time.

use std::time::SystemTime;

/// Returns the current time in milliseconds since the UNIX epoch.
///
/// Production code uses the real wall clock; tests inject
/// `DeterministicTimeSource` (see `vb_runtime::primitives::collect::tests`)
/// for reproducible scheduling.
pub trait TimeSource: Send + Sync {
    /// Returns the current time in milliseconds since the UNIX epoch.
    fn now_millis(&self) -> Result<u64, TimeError>;
}

/// Real wall-clock time source.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealTimeSource;

impl TimeSource for RealTimeSource {
    fn now_millis(&self) -> Result<u64, TimeError> {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .map_err(|_| TimeError::BeforeEpoch)
            .and_then(|ms| ms.try_into().map_err(|_| TimeError::Overflow))
    }
}

/// Deterministic time source for tests. Holds a single u64 value that
/// `now_millis` returns regardless of clock time.
#[derive(Debug, Clone, Copy)]
pub struct DeterministicTimeSource {
    millis: u64,
}

impl DeterministicTimeSource {
    #[must_use]
    pub const fn new(millis: u64) -> Self {
        Self { millis }
    }
}

impl TimeSource for DeterministicTimeSource {
    fn now_millis(&self) -> Result<u64, TimeError> {
        Ok(self.millis)
    }
}

/// Time source errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    /// System time is before UNIX epoch.
    BeforeEpoch,
    /// Milliseconds overflowed u64.
    Overflow,
}

impl core::fmt::Display for TimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BeforeEpoch => f.write_str("system time is before UNIX epoch"),
            Self::Overflow => f.write_str("millis_since_epoch overflow"),
        }
    }
}

impl std::error::Error for TimeError {}
