//! Verification pipeline domain module.
//!
//! Separates the verification domain (types, pipeline, structural gates)
//! from the CLI command layer in `verify.rs`.  The public API is:
//!
//! - [`types::VerifyOk`] — success result
//! - [`types::VerifyError`] — failure taxonomy
//! - [`pipeline::run_verification`] — the five-phase pipeline
//! - [`types::exit_code_for_error`] — error → exit-code mapping
//!
//! All other items are `pub(crate)` implementation details.

#![forbid(unsafe_code)]

pub(crate) mod types;

pub(crate) mod advisory;
pub(crate) mod pipeline;

mod tests_gate_mapping;
mod tests_pipeline;

// Re-export the public domain API at the module root for convenient import.
pub(crate) use advisory::{
    check_action_contracts, check_capability_requirements, check_idempotency,
    check_replay_determinism, check_slot_bounds, check_taint_propagation, run_advisory_gate,
};
pub(crate) use pipeline::run_verification;
pub(crate) use types::{VerifyError, VerifyOk, exit_code_for_error};
