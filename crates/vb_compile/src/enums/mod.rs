//! Enum parity tests verifying SideEffect and RetrySafety match master plan Section 65.
//!
//! These tests validate that the `SideEffect` and `RetrySafety` enums in
//! `vb_core::action` conform to the 7-variant and 4-variant taxonomies
//! defined in the master plan respectively. Per `velvet-ballistics-MASTER.md` §65:
//!
//! - `SideEffect`: Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite,
//!   Process, UnsafeShell (7 variants)
//! - `RetrySafety`: Idempotent, RequiresIdempotencyKey, NotRetrySafe, Unknown
//!   (4 variants)
//!
//! Both enums are now implemented per master §65; these tests assert the
//! live contract.

#[cfg(test)]
mod side_effect_tests;

#[cfg(test)]
mod retry_safety_tests;
