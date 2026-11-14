#![forbid(unsafe_code)]
//! Replay module for journal recovery.
//!
//! Organized into:
//! - `core`: Core replay logic (replay_events, snapshot handling)
//! - `summary`: Summary building and frame seed construction

pub mod attempt;
pub mod core;
pub mod owned_for_each;
mod owned_for_each_summary;
pub mod summary;

// Re-exports
pub use core::{
    extract_terminal, is_terminal_event, load_snapshot, recover_full_journal,
    recover_snapshot_plus_tail, replay_events,
};
pub use owned_for_each::{
    OwnedForEachReplay, OwnedForEachReplayWithCollectSnapshots, is_owned_for_each_contract,
    is_owned_input_replay_contract, replay_owned_for_each,
    replay_owned_for_each_with_collect_snapshots,
};
pub use summary::{
    RecoveryFrameSeedBuilder, apply_summary_event, recover_raw_runtime_frame_seed_from_events,
    recover_raw_runtime_frame_seed_from_events_with_workflow, recover_run_admission_from_events,
    recover_runtime_frame_seed_from_events, recover_runtime_frame_seed_from_events_with_workflow,
    summarize_recovery_events,
};
