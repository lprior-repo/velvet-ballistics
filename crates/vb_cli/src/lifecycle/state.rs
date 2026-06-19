#![forbid(unsafe_code)]
//! Lifecycle state derivation and journal replay.
//!
//! This module owns the state lookup pipeline: deriving a [`LifecycleState`]
//! from journal events and collecting per-run state snapshots.

use chrono::Utc;
use vb_core::errors::CoreError;
use vb_core::ids::RunId;
use vb_core::workflow::{LifecycleState, RunState};
use vb_storage::{EventSeq, FjallJournal, derive_lifecycle_state_from_events};

/// Result type for lifecycle operations using CoreError.
pub type LifecycleResult<T> = Result<T, CoreError>;

/// Derives the current lifecycle state for a run directly from the journal.
///
/// This is the primary state lookup used by lifecycle commands. Unlike
/// `replay()` which builds global state, this derives state for a single run
/// by reading its event sequence from the journal.
pub(super) fn current_state_from_journal(
    run: RunId,
    journal: &FjallJournal,
) -> LifecycleResult<LifecycleState> {
    let events = journal
        .events_for_run(run)
        .map_err(|e| CoreError::ReplayCorruption {
            code: CoreError::REPLAY_CORRUPTION_CODE,
            context: format!("failed to read events for run {:?}: {}", run, e),
            timestamp: Utc::now(),
            bead_id: Some(run),
        })?;
    Ok(derive_lifecycle_state_from_events(&events))
}

/// Replays the journal to reconstruct run states.
///
/// # Arguments
///
/// * `journal` - The journal to replay
///
/// # Errors
///
/// Returns `CoreError`:
/// - `ReplayCorruption` if the journal replay fails due to corruption or sequence gaps
pub fn replay(journal: &FjallJournal) -> LifecycleResult<Vec<RunState>> {
    // Enumerate all runs from the journal header keyspaces
    let headers = journal
        .run_headers()
        .map_err(|e| CoreError::ReplayCorruption {
            code: CoreError::REPLAY_CORRUPTION_CODE,
            context: format!("failed to read run headers: {}", e),
            timestamp: Utc::now(),
            bead_id: None,
        })?;

    // For each run, derive final state from event sequence and collect directly
    let mut states = Vec::new();
    for header in &headers {
        let events =
            journal
                .events_for_run(header.run)
                .map_err(|e| CoreError::ReplayCorruption {
                    code: CoreError::REPLAY_CORRUPTION_CODE,
                    context: format!("replay corruption for run {:?}: {}", header.run, e),
                    timestamp: Utc::now(),
                    bead_id: Some(header.run),
                })?;
        let lifecycle = derive_lifecycle_state_from_events(&events);
        states.push(RunState {
            run_id: header.run,
            lifecycle,
        });
    }

    Ok(states)
}

// ============================================================================
// EventSeq extension trait
// ============================================================================

/// Extension trait for EventSeq increment.
pub(super) trait EventSeqExt {
    fn increment(self) -> Self;
}

impl EventSeqExt for EventSeq {
    fn increment(self) -> Self {
        Self::new(self.get().saturating_add(1))
    }
}
