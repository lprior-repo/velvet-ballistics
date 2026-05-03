//! Replay module for journal recovery.
//!
//! Organized into:
//! - `core`: Core replay logic (replay_events, snapshot handling)
//! - `summary`: Summary building and frame seed construction

pub mod core;
pub mod summary;

// Re-exports
pub use core::{
    extract_terminal, is_terminal_event, load_snapshot, recover_full_journal,
    recover_snapshot_plus_tail, replay_events,
};
pub use summary::{
    RecoveryFrameSeedBuilder, apply_summary_event, recover_runtime_frame_seed_from_events,
    summarize_recovery_events,
};
