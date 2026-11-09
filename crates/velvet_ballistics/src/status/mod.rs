//! Status derivation and replay timeline for CLI output.
//!
//! This module provides:
//! - Pure status derivation from journal events (no runtime shard access)
//! - Replay timeline explanation with snapshot boundaries and journal tails
//! - Typed error diagnostics for missing runs and index inconsistencies

#![forbid(unsafe_code)]

use vb_core::ids::{ActionId, RunId, StepIdx, WorkflowDigest};
use vb_storage::JournalEvent;

/// Errors that can occur when deriving run or workflow status.
///
/// `#[non_exhaustive]` - new variants may be added in the future.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusError {
    /// The requested run was not found in the journal.
    RunNotFound {
        /// The run identifier that was not found.
        run_id: RunId,
    },
    /// An inconsistency was detected in the journal (e.g., stale pending action).
    Inconsistency {
        /// Description of the inconsistency.
        reason: String,
    },
}

/// Derived status computed from journal events.
///
/// This is a pure computation that does not access the runtime shard or YAML source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedStatus {
    /// Run is pending (no events yet).
    Pending,
    /// Run is actively executing.
    Active,
    /// Run is waiting on an external action to complete.
    WaitingAction {
        /// The pending action identifier.
        pending_action: ActionId,
        /// The step that scheduled the action.
        pending_step: StepIdx,
    },
    /// Run is waiting on an external answer (ask/wait).
    WaitingAnswer {
        /// The step waiting for an answer.
        pending_step: StepIdx,
    },
    /// Run is completed successfully.
    Completed,
    /// Run has failed.
    Failed,
    /// Run was cancelled.
    Cancelled,
    /// Run failed and has a retry timer active.
    BackingOff {
        /// The step to retry from.
        retry_step: StepIdx,
    },
    /// An inconsistency was detected in the journal state.
    Inconsistency(String),
}

/// Entry in the replay timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayExplainEntry {
    /// Sequence number of this event.
    pub seq: u64,
    /// The event type name.
    pub event_type: &'static str,
    /// Workflow digest if available (for RunAccepted).
    pub workflow_digest: Option<WorkflowDigest>,
    /// Record kind if available.
    pub record_kind: Option<vb_storage::RecordKind>,
    /// Step index if available.
    pub step: Option<u16>,
    /// Action identifier if available.
    pub action: Option<u16>,
}

/// Snapshot boundary information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotBoundary {
    /// Sequence number at the snapshot boundary.
    pub seq: u64,
    /// The run identifier.
    pub run_id: RunId,
}

/// Timeline for a single run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReplayTimeline {
    /// The run identifier.
    pub run_id: RunId,
    /// Snapshot boundary marker (if a snapshot exists).
    pub snapshot_boundary: Option<SnapshotBoundary>,
    /// Journal tail entries after the snapshot.
    pub entries: Vec<ReplayExplainEntry>,
}

/// Replay timeline result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayTimeline {
    /// Valid timeline with run entries.
    Valid {
        /// All runs with their timelines.
        runs: Vec<RunReplayTimeline>,
    },
    /// Empty journal (no runs).
    Empty,
}

/// Derive status from journal events.
///
/// This is a pure function that computes status without accessing
/// YAML source or runtime state.
///
/// # Arguments
///
/// * `events` - The journal events for a run
///
/// # Returns
///
/// The derived status for the run
#[must_use]
pub fn derive_status_from_events(events: &[JournalEvent]) -> DerivedStatus {
    // Empty events means Pending
    if events.is_empty() {
        return DerivedStatus::Pending;
    }

    // Find the last event and scan for pending actions
    let mut pending_action: Option<ActionId> = None;
    let mut pending_step: Option<StepIdx> = None;
    let mut retry_step: Option<StepIdx> = None;
    let mut terminal_state: Option<DerivedStatus> = None;

    for event in events {
        match event {
            JournalEvent::RunCancelled { .. } => {
                terminal_state = Some(DerivedStatus::Cancelled);
            }
            JournalEvent::RunFinished { .. } => {
                terminal_state = Some(DerivedStatus::Completed);
            }
            JournalEvent::RunFailedEvent { .. } => {
                // Will check for retry later
            }
            JournalEvent::RetryScheduledEvent { step, .. } => {
                retry_step = Some(*step);
            }
            JournalEvent::ActionScheduled { action, step, .. } if pending_action.is_none() => {
                pending_action = Some(*action);
                pending_step = Some(*step);
            }
            JournalEvent::ActionScheduledTicket { ticket, .. } if pending_action.is_none() => {
                pending_action = Some(ticket.action);
                pending_step = Some(ticket.step);
            }
            JournalEvent::AskScheduledEvent { step, .. }
            | JournalEvent::WaitScheduledEvent { step, .. }
                if pending_action.is_none() && terminal_state.is_none() =>
            {
                return DerivedStatus::WaitingAnswer {
                    pending_step: *step,
                };
            }
            _ => {}
        }
    }

    // If we have a terminal state and it's not overridden by pending actions, return it
    if let Some(status) = terminal_state {
        // Check for inconsistency: terminal state with stale pending action
        if let (Some(_), DerivedStatus::Completed) = (pending_action, &status) {
            // Completed with pending action is an inconsistency
            return DerivedStatus::Inconsistency(
                "stale pending action after completed run".to_string(),
            );
        }
        return status;
    }

    // Check if run failed
    let has_failed = events
        .iter()
        .any(|e| matches!(e, JournalEvent::RunFailedEvent { .. }));

    // If failed with retry timer, return BackingOff
    if has_failed {
        if let Some(step) = retry_step {
            return DerivedStatus::BackingOff { retry_step: step };
        }
        return DerivedStatus::Failed;
    }

    // If we have a pending action, return WaitingAction
    if let (Some(action), Some(step)) = (pending_action, pending_step) {
        return DerivedStatus::WaitingAction {
            pending_action: action,
            pending_step: step,
        };
    }

    // Otherwise, return Active
    DerivedStatus::Active
}

pub mod replay;

pub use replay::{replay_explain, replay_explain_for_run};

#[cfg(test)]
mod tests;
