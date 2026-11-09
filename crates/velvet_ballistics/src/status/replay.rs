//! Replay timeline explanation with snapshot boundaries and journal tails.

use vb_core::ids::RunId;
use vb_storage::{FjallJournal, JournalEvent};

use super::{ReplayExplainEntry, ReplayTimeline, RunReplayTimeline, SnapshotBoundary, StatusError};

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
        let events =
            journal
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
        JournalEvent::ActionScheduled {
            step: s, action: a, ..
        } => (
            "ActionScheduled",
            None,
            Some(vb_storage::RecordKind::ActionScheduled),
            Some(s.get()),
            Some(a.get()),
        ),
        JournalEvent::ActionCompletedEvent {
            step: s, action: a, ..
        } => (
            "ActionCompleted",
            None,
            Some(vb_storage::RecordKind::ActionCompleted),
            Some(s.get()),
            Some(a.get()),
        ),
        JournalEvent::ActionFailedEvent {
            step: s, action: a, ..
        } => (
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
        _ => ("Unknown", None, None, None, None),
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
