#![forbid(unsafe_code)]

//! Contract violation types for ResourceContract validation against profiles.

use thiserror::Error;

/// Error returned when a ResourceContract exceeds a profile or hard limit.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ContractViolation {
    /// A contract field exceeds the profile limit.
    #[error("field '{field}' value {actual} exceeds profile limit {profile_limit}")]
    ExceedsProfileLimit {
        /// Contract field name.
        field: &'static str,
        /// Actual contract value.
        actual: u64,
        /// Profile limit for that field.
        profile_limit: u64,
    },

    /// A contract field exceeds the hard limit from limits.rs.
    #[error("field '{field}' value {actual} exceeds hard limit {hard_limit}")]
    ExceedsHardLimit {
        /// Contract field name.
        field: &'static str,
        /// Actual contract value.
        actual: u64,
        /// Hard limit from limits.rs.
        hard_limit: u64,
    },
}
