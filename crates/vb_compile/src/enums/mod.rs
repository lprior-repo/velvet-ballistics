//! Enum parity tests verifying SideEffect and RetrySafety match master plan Section 65.
//!
//! These tests validate that the `SideEffect` and `RetrySafety` enums in
//! `vb_core::action` conform to the 7-variant and 4-variant taxonomies
//! defined in the master plan respectively.
//!
//! **FIXME(MAJOR-6):** The current implementation in `vb_core/src/action.rs`
//! uses a BROKEN 5-variant `SideEffect` (None, Writes, Sends, Creates, Destroys)
//! and a BROKEN 3-variant `RetrySafety` (Safe, KeyRequired, Unsafe). The master
//! plan Section 65 defines the CORRECT taxonomies:
//!
//! - `SideEffect`: Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite,
//!   Process, UnsafeShell (7 variants)
//! - `RetrySafety`: Idempotent, RequiresIdempotencyKey, NotRetrySafe, Unknown
//!   (4 variants)
//!
//! These tests assert the FIXED state. They will NOT compile until the
//! implementation is corrected.

#[cfg(test)]
mod side_effect_tests;

#[cfg(test)]
mod tests;
