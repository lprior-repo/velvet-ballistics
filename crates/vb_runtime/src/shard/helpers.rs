#![forbid(unsafe_code)]
//! Pure helper functions for shard operations.
//!
//! This module re-exports from focused submodules:
//! - `action`: Action and ticket validation
//! - `error`: Error handler lookup
//! - `retry`: Retry policy and attempt tracking
//! - `snapshot`: State snapshot creation
//! - `timer`: Timer registration and advancement

pub mod action;
pub mod error;
pub mod retry;
pub mod snapshot;
pub mod timer;

pub use action::{
    action_input_slot, action_output_slot, advance_after_action_completion, make_run_state,
    new_action_attempts, normalize_scheduled_ticket, record_scheduled_attempt, seed_input_slots,
    validate_action_completion,
};
pub use error::{find_error_handler_for_failure, result_slot_for_finished_run};
pub use retry::{record_retry_attempt, retry_metadata_exists, retry_policy_after_action};
pub use snapshot::snapshot_from_state;
pub use timer::{advance_after_timer_fire, timer_registration_required};

#[cfg(test)]
#[path = "helpers/tests.rs"]
mod tests;
