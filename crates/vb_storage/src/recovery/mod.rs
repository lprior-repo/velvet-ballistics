//! Recovery module for velvet-ballastics journal.
//!
//! Organized into:
//! - `types`: Recovery error types and state types
//! - `replay`: Core replay logic and event processing
//! - `recover`: High-level recovery orchestration
//!
//! Provides:
//! - Digest mismatch detection (workflow source, compiled IR, action ABI, policy)
//! - Full primitive replay (all node kinds)
//! - Non-idempotent action policy: block re-execution during recovery
//! - Replay divergence detection with typed error
//! - Snapshot-plus-tail journal recovery
//! - Full journal recovery when no snapshot available

pub mod types;
pub mod replay;
pub mod recover;

#[cfg(test)]
mod tests;

// ============================================================================
// Re-exports - types
// ============================================================================

pub use types::{
    ActionReplayTracker, DigestCheck, RecoveryError, RecoveryFrameSeed, RecoveryHydration,
    RecoveryResult, RecoveryRuntimeSummary, RecoveryTerminalState, RecoveredStepEntry,
    RecoveredStepState, RunSnapshot, UnsupportedRecoveryState,
};

// ============================================================================
// Re-exports - replay
// ============================================================================

pub use replay::{
    extract_terminal, is_terminal_event, recover_full_journal, load_snapshot,
    recover_snapshot_plus_tail, replay_events, summarize_recovery_events,
    recover_runtime_frame_seed_from_events,
};

// ============================================================================
// Re-exports - recover
// ============================================================================

pub use recover::{
    check_workflow_source_digest, check_compiled_ir_digest, verify_digests,
    recover_runtime_summary, recover_runtime_frame_seed, recover_all_incomplete_runs,
};
