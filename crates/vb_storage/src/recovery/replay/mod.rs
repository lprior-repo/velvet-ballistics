#![forbid(unsafe_code)]
//! Replay module for journal recovery.
//!
//! Organized into:
//! - `core`: Core replay logic (replay_events, sequence validation)
//! - `recovery_ops`: Full journal replay and snapshot-based recovery
//! - `terminal`: Terminal event detection
//! - `summary`: Summary building and frame seed construction

mod action_abi;
pub mod admission;
pub mod attempt;
mod core;
mod recovery_ops;
pub mod summary;
mod terminal;

// Re-exports from core
pub use core::replay_events;

// Re-exports from attempt (forwarded for backward compat)
pub use attempt::{
    replay_attempt_is_current, replay_attempt_is_stale, replay_attempt_or_default,
    replay_event_has_state_effect, replay_event_is_stale_state_effect, replay_step_order_diverges,
};

// Re-exports from recovery_ops
pub use recovery_ops::{load_snapshot, recover_full_journal, recover_snapshot_plus_tail};

// Re-exports from terminal
pub use terminal::{extract_terminal, is_terminal_event};

// Re-exports from summary
pub use summary::{
    RecoveryFrameSeedBuilder, apply_summary_event, recover_runtime_frame_seed_from_events,
    recover_runtime_frame_seed_from_events_with_workflow, summarize_recovery_events,
};
