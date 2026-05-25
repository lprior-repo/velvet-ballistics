//! Status derivation and replay timeline for CLI output.
//!
//! This module provides:
//! - Pure status derivation from journal events (no runtime shard access)
//! - Replay timeline explanation with snapshot boundaries and journal tails
//! - Typed error diagnostics for missing runs and index inconsistencies

#![forbid(unsafe_code)]

use vb_core::ids::{ActionId, RunId, StepIdx, WorkflowDigest};
use vb_storage::{FjallJournal, JournalEvent};

/// Errors produced by status derivation and replay explain.
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
            JournalEvent::AskScheduledEvent { step, .. } | JournalEvent::WaitScheduledEvent { step, .. }
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
    let has_failed = events.iter().any(|e| matches!(e, JournalEvent::RunFailedEvent { .. }));

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

/// Explain the replay process for a journal.
///
/// Returns a timeline showing snapshot boundaries and journal tail events.
///
/// # Arguments
///
/// * `journal` - The journal to explain
///
/// # Returns
///
/// The replay timeline or an error
pub fn replay_explain(journal: &FjallJournal) -> Result<ReplayTimeline, StatusError> {
    let headers = journal
        .run_headers()
        .map_err(|e| StatusError::Inconsistency {
            reason: format!("failed to read run headers: {}", e),
        })?;

    if headers.is_empty() {
        return Ok(ReplayTimeline::Empty);
    }

    let mut runs = Vec::new();
    for header in &headers {
        let events = journal
            .events_for_run(header.run)
            .map_err(|e| StatusError::Inconsistency {
                reason: format!("failed to read events for run {:?}: {}", header.run, e),
            })?;

        let timeline = build_run_timeline(header.run, &events);
        runs.push(timeline);
    }

    Ok(ReplayTimeline::Valid { runs })
}

/// Explain replay for a specific run.
///
/// # Arguments
///
/// * `journal` - The journal
/// * `run` - The run identifier to explain
///
/// # Returns
///
/// The run's replay timeline or an error
pub fn replay_explain_for_run(
    journal: &FjallJournal,
    run: RunId,
) -> Result<RunReplayTimeline, StatusError> {
    // Check if run exists
    let headers = journal
        .run_headers()
        .map_err(|e| StatusError::Inconsistency {
            reason: format!("failed to read run headers: {}", e),
        })?;

    let header = headers
        .iter()
        .find(|h| h.run == run)
        .ok_or(StatusError::RunNotFound { run_id: run })?;

    let events = journal
        .events_for_run(run)
        .map_err(|e| StatusError::Inconsistency {
            reason: format!("failed to read events for run {:?}: {}", run, e),
        })?;

    Ok(build_run_timeline(header.run, &events))
}

fn build_run_timeline(run_id: RunId, events: &[JournalEvent]) -> RunReplayTimeline {
    let mut entries = Vec::new();
    let mut snapshot_boundary: Option<SnapshotBoundary> = None;

    for event in events {
        let entry = build_explain_entry(event);
        // First entry is the snapshot boundary
        if snapshot_boundary.is_none() {
            snapshot_boundary = Some(SnapshotBoundary {
                seq: entry.seq,
                run_id,
            });
        }
        entries.push(entry);
    }

    RunReplayTimeline {
        run_id,
        snapshot_boundary,
        entries,
    }
}

fn build_explain_entry(event: &JournalEvent) -> ReplayExplainEntry {
    let seq = event.seq().get();
    let (event_type, workflow_digest, record_kind, step, action) = match event {
        JournalEvent::RunAccepted { workflow, .. } => (
            "RunAccepted",
            Some(*workflow),
            Some(vb_storage::RecordKind::RunAccepted),
            None,
            None,
        ),
        JournalEvent::RunAdmission {
            artifact_digest, ..
        } => (
            "RunAdmission",
            Some(*artifact_digest),
            Some(vb_storage::RecordKind::RunAdmission),
            None,
            None,
        ),
        JournalEvent::StepStarted { step: s, .. } => (
            "StepStarted",
            None,
            Some(vb_storage::RecordKind::StepStarted),
            Some(s.get()),
            None,
        ),
        JournalEvent::StepSucceeded { step: s, .. } => (
            "StepSucceeded",
            None,
            Some(vb_storage::RecordKind::SlotWritten),
            Some(s.get()),
            None,
        ),
        JournalEvent::ActionScheduled { step: s, action: a, .. } => (
            "ActionScheduled",
            None,
            Some(vb_storage::RecordKind::ActionScheduled),
            Some(s.get()),
            Some(a.get()),
        ),
        JournalEvent::ActionCompletedEvent { step: s, action: a, .. } => (
            "ActionCompleted",
            None,
            Some(vb_storage::RecordKind::ActionCompleted),
            Some(s.get()),
            Some(a.get()),
        ),
        JournalEvent::ActionFailedEvent { step: s, action: a, .. } => (
            "ActionFailed",
            None,
            Some(vb_storage::RecordKind::ActionFailed),
            Some(s.get()),
            Some(a.get()),
        ),
        JournalEvent::WaitScheduledEvent { step: s, .. } => (
            "WaitScheduled",
            None,
            Some(vb_storage::RecordKind::WaitScheduled),
            Some(s.get()),
            None,
        ),
        JournalEvent::AskScheduledEvent { step: s, .. } => (
            "AskScheduled",
            None,
            Some(vb_storage::RecordKind::AskScheduled),
            Some(s.get()),
            None,
        ),
        JournalEvent::AskAnsweredEvent { step: s, .. } => (
            "AskAnswered",
            None,
            Some(vb_storage::RecordKind::AskAnswered),
            Some(s.get()),
            None,
        ),
        JournalEvent::RetryScheduledEvent { step: s, .. } => (
            "RetryScheduled",
            None,
            Some(vb_storage::RecordKind::RetryScheduled),
            Some(s.get()),
            None,
        ),
        JournalEvent::RunCancelled { .. } => (
            "RunCancelled",
            None,
            Some(vb_storage::RecordKind::RunCancelled),
            None,
            None,
        ),
        JournalEvent::RunFinished { .. } => (
            "RunFinished",
            None,
            Some(vb_storage::RecordKind::RunFinished),
            None,
            None,
        ),
        JournalEvent::RunFailedEvent { .. } => (
            "RunFailed",
            None,
            Some(vb_storage::RecordKind::RunFailed),
            None,
            None,
        ),
        JournalEvent::RunResumed { .. } => (
            "RunResumed",
            None,
            Some(vb_storage::RecordKind::RunResumed),
            None,
            None,
        ),
        JournalEvent::RunRetried { .. } => (
            "RunRetried",
            None,
            Some(vb_storage::RecordKind::RunRetried),
            None,
            None,
        ),
        JournalEvent::RunAnswered { .. } => (
            "RunAnswered",
            None,
            Some(vb_storage::RecordKind::RunAnswered),
            None,
            None,
        ),
        JournalEvent::SlotWrittenEvent { slot: _s, .. } => (
            "SlotWritten",
            None,
            Some(vb_storage::RecordKind::SlotWritten),
            None,
            None,
        ),
        JournalEvent::ActionScheduledTicket { ticket, .. } => (
            "ActionScheduledTicket",
            None,
            Some(vb_storage::RecordKind::ActionScheduled),
            Some(ticket.step.get()),
            Some(ticket.action.get()),
        ),
        JournalEvent::ActionCompletedEnvelope { ticket, .. } => (
            "ActionCompletedEnvelope",
            None,
            Some(vb_storage::RecordKind::ActionCompleted),
            Some(ticket.step.get()),
            Some(ticket.action.get()),
        ),
        _ => (
            "Unknown",
            None,
            None,
            None,
            None,
        ),
    };

    ReplayExplainEntry {
        seq,
        event_type,
        workflow_digest,
        record_kind,
        step,
        action,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::RunId;
    use vb_core::workflow::WorkflowDigest;

    fn dummy_digest() -> WorkflowDigest {
        WorkflowDigest::from_bytes([0xAB_u8; 32])
    }

    #[test]
    fn derive_status_empty_events_returns_pending() {
        let events: Vec<JournalEvent> = vec![];
        let status = derive_status_from_events(&events);
        assert_eq!(status, DerivedStatus::Pending);
    }

    #[test]
    fn derive_status_run_accepted_is_active() {
        let events = vec![JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: vb_storage::EventSeq::ZERO,
            workflow: dummy_digest(),
        }];
        let status = derive_status_from_events(&events);
        assert_eq!(status, DerivedStatus::Active);
    }

    #[test]
    fn derive_status_action_scheduled_is_waiting_action() {
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: vb_storage::EventSeq::ZERO,
                workflow: dummy_digest(),
            },
            JournalEvent::ActionScheduled {
                run: RunId::new(1),
                seq: vb_storage::EventSeq::new(1),
                step: StepIdx::new(2),
                action: ActionId::new(5),
                attempt: 1,
            },
        ];
        let status = derive_status_from_events(&events);
        match status {
            DerivedStatus::WaitingAction {
                pending_action,
                pending_step,
            } => {
                assert_eq!(pending_action, ActionId::new(5));
                assert_eq!(pending_step, StepIdx::new(2));
            }
            other => panic!("expected WaitingAction, got {:?}", other),
        }
    }

    #[test]
    fn derive_status_run_finished_is_completed() {
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: vb_storage::EventSeq::ZERO,
                workflow: dummy_digest(),
            },
            JournalEvent::RunFinished {
                run: RunId::new(1),
                seq: vb_storage::EventSeq::new(1),
                result: vb_core::ids::SlotIdx::new(0),
                attempt: 1,
            },
        ];
        let status = derive_status_from_events(&events);
        assert_eq!(status, DerivedStatus::Completed);
    }

    #[test]
    fn derive_status_run_failed_with_retry_is_backing_off() {
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: vb_storage::EventSeq::ZERO,
                workflow: dummy_digest(),
            },
            JournalEvent::RunFailedEvent {
                run: RunId::new(1),
                seq: vb_storage::EventSeq::new(1),
                attempt: 1,
            },
            JournalEvent::RetryScheduledEvent {
                run: RunId::new(1),
                seq: vb_storage::EventSeq::new(2),
                step: StepIdx::new(1),
                attempt: 1,
            },
        ];
        let status = derive_status_from_events(&events);
        match status {
            DerivedStatus::BackingOff { retry_step } => {
                assert_eq!(retry_step, StepIdx::new(1));
            }
            other => panic!("expected BackingOff, got {:?}", other),
        }
    }
}
