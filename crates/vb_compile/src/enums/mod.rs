//! Enum parity tests verifying SideEffect and RetrySafety match master plan Section 65.
//!
//! These tests validate that the `SideEffect` and `RetrySafety` enums in
//! `vb_core::action` conform to the 7-variant and 4-variant taxonomies
//! defined in the master plan respectively.
//!
//! The implementation in `vb_core/src/action.rs` is expected to expose the
//! master-plan Section 65 taxonomies:
//!
//! - `SideEffect`: Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite,
//!   Process, UnsafeShell (7 variants)
//! - `RetrySafety`: Idempotent, RequiresIdempotencyKey, NotRetrySafe, Unknown
//!   (4 variants)
//!
//! These tests assert the current contract stays aligned with that taxonomy.

#[cfg(test)]
mod side_effect_tests;

#[cfg(test)]
mod tests;
