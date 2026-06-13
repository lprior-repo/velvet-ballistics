#![forbid(unsafe_code)]
//! Recovery module for velvet-ballistics journal.
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

pub(crate) mod digest;
pub mod hydrate;
pub mod hydrate_support;
pub mod recover;
pub mod replay;
pub mod types;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tests/error_recovery_tests.rs"]
mod error_recovery_tests;

#[cfg(test)]
mod vb_h6ix_tests;

#[cfg(test)]
mod recovery_unit_tests;

#[cfg(test)]
mod property_tests;

// ============================================================================
// Re-exports - types
// ============================================================================

pub use types::{
    ActionReplayTracker, DigestCheck, DigestCheckConfig, RecoveredPendingAction,
    RecoveredRunAdmission, RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState,
    RecoveryError, RecoveryFrameSeed, RecoveryHydration, RecoveryResult, RecoveryRuntimeSummary,
    RecoveryTerminalState, RunSnapshot, UnsupportedRecoveryState,
};

// ============================================================================
// Re-exports - replay
// ============================================================================

pub use replay::{
    RecoveryFrameSeedBuilder, extract_terminal, is_terminal_event, load_snapshot,
    recover_full_journal, recover_runtime_frame_seed_from_events,
    recover_runtime_frame_seed_from_events_with_workflow, recover_snapshot_plus_tail,
    replay_events, summarize_recovery_events,
};

// ============================================================================
// Re-exports - recover
// ============================================================================

pub use hydrate::{hydrate_run_frame, hydrate_run_frame_from_events};
pub use recover::{
    check_action_abi_digest, check_action_abi_digests, check_compiled_ir_digest,
    check_policy_digest, check_policy_digests, check_workflow_source_digest,
    recover_all_incomplete_runs, recover_runtime_frame_seed, recover_runtime_summary,
    recover_runtime_summary_with_expected, verify_digests, write_recovered_snapshot,
};
