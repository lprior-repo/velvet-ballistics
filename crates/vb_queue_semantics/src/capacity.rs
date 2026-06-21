//! Capacity boundary: validated bounded-queue capacity domain.
//!
//! Defines the trusted capacity range and its rejection taxonomy. Core
//! transition functions accept only `QueueState` — they never observe raw
//! `usize` capacity again.

/// Shared queue capacity maximum used by the Verus-native helper route.
pub const SHARED_QUEUE_CAPACITY_MAX: usize = 65_536;

/// Reason a bounded queue capacity was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityRejection {
    /// Capacity must be non-zero.
    Zero,
    /// Capacity exceeds the caller-supplied maximum.
    AboveMaximum { maximum: usize },
}

/// Validates a bounded queue capacity against a caller-owned maximum.
pub const fn validate_capacity(capacity: usize, maximum: usize) -> Result<(), CapacityRejection> {
    if capacity == 0 {
        return Err(CapacityRejection::Zero);
    }
    if capacity > maximum {
        return Err(CapacityRejection::AboveMaximum { maximum });
    }
    Ok(())
}

/// Verus-shared helper route for the accepted 1..=65536 capacity domain.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize) -> bool[capacity > 0 && capacity <= 65536]))]
pub const fn helper_valid_capacity(capacity: usize) -> bool {
    capacity > 0 && capacity <= SHARED_QUEUE_CAPACITY_MAX
}

#[cfg(test)]
mod tests;
