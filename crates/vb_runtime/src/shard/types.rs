#![forbid(unsafe_code)]
//! Single-threaded shard owning mutable run state directly.
//!
//! This module re-exports types from submodules for backward compatibility.
//! The actual type definitions live in:
//! - `timer.rs` – TimerTick, TimerDuration, TimerDeadline, TimerKind, PendingTimer
//! - `command.rs` – ShardCommand
//! - `ask.rs` – AskTicket, AskAnswer
//! - `run_state.rs` – RunState, RuntimeState, RuntimeEvent, ResumeStatus,
//!   ResumeResult, ResumeError, InspectSnapshot, InspectResponse
//! - `introspection.rs` – IntrospectionRegistry, InspectHandle, InspectSnapshotFormatter,
//!   UnregisterOutcome, RegisterOverlapOutcome
//! - `queue.rs` – ShardCommandQueue, MAX_COMMAND_QUEUE_CAPACITY
//! - `config.rs` – Shard, ShardConfig, ShardStatus, ShardHealth

// Aggregate resource model touchpoints for vb-qi37.2.1:
// ShardConfig aggregate_capacity, Shard active_usage, Shard reservations,
// RunState AggregateReservation, ShardStatus active_usage aggregate_capacity.

// ============================================================================
// Re-exports from timer.rs
// ============================================================================

pub use super::timer::{
    PendingTimer, PendingTimerKind, TimerDeadline, TimerDuration, TimerKind, TimerTick,
};

// ============================================================================
// Re-exports from command.rs
// ============================================================================

pub use super::command::ShardCommand;

// ============================================================================
// Re-exports from ask.rs
// ============================================================================

pub use super::ask::{AskAnswer, AskTicket};

// ============================================================================
// Re-exports from run_state.rs
// ============================================================================

pub use super::run_state::{
    InspectResponse, InspectSnapshot, ResumeError, ResumeResult, ResumeStatus, RunState,
    RuntimeEvent, RuntimeState, TerminalOutcome,
};

// ============================================================================
// Re-exports from introspection.rs
// ============================================================================

pub use super::introspection::{
    InspectHandle, InspectSnapshotFormatter, IntrospectionRegistry, RegisterOverlapOutcome,
    UnregisterOutcome,
};

// ============================================================================
// Re-exports from queue.rs
// ============================================================================

pub use super::queue::{
    MAX_COMMAND_QUEUE_CAPACITY, ShardCommandQueue, is_valid_command_queue_capacity,
};

// ============================================================================
// Re-exports from config.rs
// ============================================================================

pub use super::config::{
    Shard, ShardConfig, ShardHealth, ShardStatus, is_valid_step_budget_per_tick,
    is_valid_trace_capacity,
};
