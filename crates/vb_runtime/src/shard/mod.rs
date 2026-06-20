#![forbid(unsafe_code)]
//! Single-threaded shard owning mutable run state directly.

pub mod arena;
pub mod ask;
pub mod command;
pub mod completion_watermark;
pub mod config;
pub mod directive;
pub mod helpers;
pub mod impl_;
pub mod introspection;
pub mod lifecycle;
pub mod lru_ring;
#[cfg(test)]
mod lru_ring_red_queen_tests;
#[cfg(test)]
mod lru_ring_tests;
#[cfg(test)]
pub mod property_tests;
pub mod queue;
pub mod run_state;
pub mod tests;
pub mod timer;
pub mod timer_wheel;
pub mod transitions;
pub mod types;

pub use completion_watermark::{CompletionDrain, CompletionWatermark, CompletionWatermarkError};
pub use directive::ShardDirective;
pub use lru_ring::{
    DEFAULT_MAX_TERMINAL_RUNS, DEFAULT_TERMINAL_RUNS_TTL_TICKS, LruRing, LruRingCounters,
};
pub use types::{
    AskAnswer, AskTicket, InspectHandle, InspectResponse, InspectSnapshot,
    InspectSnapshotFormatter, IntrospectionRegistry, MAX_COMMAND_QUEUE_CAPACITY, PendingTimer,
    PendingTimerKind, RegisterOverlapOutcome, ResumeError, ResumeResult, ResumeStatus, RunState,
    RuntimeEvent, RuntimeState, Shard, ShardCommand, ShardConfig, ShardHealth, ShardStatus,
    TerminalOutcome, TimerDeadline, TimerDuration, TimerKind, TimerTick, UnregisterOutcome,
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
