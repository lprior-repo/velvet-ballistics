#![forbid(unsafe_code)]
//! Single-threaded shard owning mutable run state directly.

pub mod completion_watermark;
pub mod directive;
pub mod contract_validation;
pub mod helpers;
pub mod impl_;
pub mod lifecycle;
pub mod snapshot;
pub mod tests;
pub mod timer_wheel;
pub mod transitions;
pub mod types;

pub use completion_watermark::{CompletionDrain, CompletionWatermark, CompletionWatermarkError};
pub use directive::ShardDirective;
pub use snapshot::{
    PendingActionBoundarySnapshot, PendingAskBoundarySnapshot, PendingAskTimeoutBoundarySnapshot,
    PendingTimerBoundarySnapshot, ShardPendingBoundarySnapshot,
};
pub use types::{
    AskAnswer, AskTicket, InspectHandle, InspectResponse, InspectSnapshot,
    InspectSnapshotFormatter, IntrospectionRegistry, MAX_COMMAND_QUEUE_CAPACITY, PendingTimer,
    PendingTimerKind, RegisterOverlapOutcome, ResumeError, ResumeResult, ResumeStatus, RunState,
    RuntimeEvent, RuntimeState, Shard, ShardCommand, ShardConfig, ShardHealth, ShardStatus,
    TimerDeadline, TimerDuration, TimerKind, TimerTick, UnregisterOutcome,
};


/// Computes ABI digest for action from run state.
pub(crate) fn compute_abi_digest_from_state(
    state: &RunState,
    action: vb_core::ids::ActionId,
) -> crate::RuntimeResult<vb_core::ids::WorkflowDigest> {
    state
        .action_contracts
        .iter()
        .find(|c| c.id == action)
        .ok_or(crate::RuntimeError::UnsupportedOperation {
            operation: "action_contract_not_found",
        })?;
    crate::recovery::action_contract_abi_digest(
        state.action_contracts.iter().find(|c| c.id == action).unwrap(),
    )
}

// Re-export vb_core types needed by tests
pub use vb_core::ids::RunId;

// Re-export helpers for tests
pub use helpers::{
    advance_after_action_completion, advance_after_timer_fire, find_error_handler_for_failure,
    new_action_attempts, record_retry_attempt, record_scheduled_attempt,
    result_slot_for_finished_run, retry_metadata_exists, retry_policy_after_action,
    seed_input_slots, snapshot_from_state, timer_registration_required, validate_action_completion,
};
