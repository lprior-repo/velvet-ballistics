#![forbid(unsafe_code)]
//! Replay module for journal recovery.
//!
//! Organized into:
//! - `core`: Core replay logic (replay_events, snapshot handling)
//! - `summary`: Summary building and frame seed construction
//! - `observation`: Semantic observation normalization for diff comparison

pub mod attempt;
pub mod core;
pub mod observation;
pub mod summary;

// Re-exports
pub use core::{
    extract_terminal, is_terminal_event, load_snapshot, recover_full_journal,
    recover_snapshot_plus_tail, replay_events,
};
pub use observation::{
    semantic_observation_signature, semantic_observations, JournalObservation,
    JournalObservationSignature, ObservationSignatureError,
};
pub use summary::{
    RecoveryFrameSeedBuilder, apply_summary_event, recover_raw_runtime_frame_seed_from_events,
    recover_raw_runtime_frame_seed_from_events_with_workflow, recover_run_admission_from_events,
    recover_runtime_frame_seed_from_events, recover_runtime_frame_seed_from_events_with_workflow,
    summarize_recovery_events,
};
