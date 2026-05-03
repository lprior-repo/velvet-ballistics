//! Single-threaded shard owning mutable run state directly.

pub mod helpers;
pub mod impl_;
pub mod lifecycle;
pub mod tests;
pub mod transitions;
pub mod types;

pub use types::{
    AskAnswer, AskTicket, InspectResponse, InspectSnapshot, MAX_COMMAND_QUEUE_CAPACITY, RunState,
    Shard, ShardCommand, ShardConfig,
};

// Re-export vb_core types needed by tests
pub use vb_core::ids::RunId;

// Re-export helpers for tests
pub use helpers::{
    advance_after_action_completion, advance_after_timer_fire, find_error_handler_for_failure,
    new_action_attempts, record_retry_attempt, record_scheduled_attempt,
    result_slot_for_finished_run, retry_metadata_exists, retry_policy_after_action,
    seed_input_slots, snapshot_from_state, timer_registration_required, validate_action_completion,
};
