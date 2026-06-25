#![forbid(unsafe_code)]

//! Deterministic drive loop for the runtime engine.
//!
//! Split into focused submodules:
//! - [`actions`]: `SlotWritten` evidence emission, collect-slot lookup,
//!   and `TogetherStart` branch counting.
//! - [`loop`]: top-level `drive_deterministic_full` and
//!   `drive_with_actions` orchestration.
//! - [`recovery`]: drive-state object that records evidence gaps so
//!   `read_slot` errors are surfaced instead of swallowed.
//! - [`timers`]: step-budget consumption helpers.
//! - [`transitions`]: `begin_drive_step` / `finish_drive_step` and
//!   signal classification.

pub(crate) mod actions;
pub(crate) mod loop_step;
pub(crate) mod recovery;
pub(crate) mod timers;
pub(crate) mod transitions;

pub use loop_step::{drive_deterministic_full, drive_with_actions};

#[doc(hidden)]
pub(crate) use actions::{compute_max_parallel_in_flight, emit_slot_evidence};
#[doc(hidden)]
pub(crate) use recovery::DriveState;

#[cfg(test)]
mod tests;