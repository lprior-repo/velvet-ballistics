#![forbid(unsafe_code)]

//! Profile validation errors for the runtime limits profile matrix.

use thiserror::Error;

/// Error returned when a profile value exceeds a hard limit.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ProfileValidationError {
    /// A profile field exceeds the corresponding hard limit.
    #[error("field '{field}' value {value} exceeds hard limit {limit}")]
    ExceedsHardLimit {
        /// Field name that violated the limit.
        field: &'static str,
        /// Actual value provided.
        value: u64,
        /// Corresponding hard limit from limits.rs.
        limit: u64,
    },

    /// A profile field was zero or negative.
    #[error("field '{field}' must be positive, got {value}")]
    ZeroValue {
        /// Field name that was zero.
        field: &'static str,
        /// Actual value provided.
        value: u64,
    },
}
