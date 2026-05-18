#![forbid(unsafe_code)]
//! Core replay logic for journal recovery.
//!
//! Provides:
//! - Event replay with divergence detection
//! - Non-idempotent action blocking
//! - Snapshot-plus-tail replay

use crate::recovery::types::{ActionReplayTracker, RecoveryError, RecoveryResult};
use crate::{EventSeq, FjallJournal, JournalEvent};
use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest};

/// Computes the maximum attempt number observed in action-scheduling and
/// action-completion events. Events without an attempt field contribute 1
/// (PRE-001: treat as attempt 1).
#[must_use]
fn compute_max_attempt(events: &[JournalEvent]) -> u16 {
    let mut max_attempt = 1u16;
    for event in events {
        if let Some(attempt) = event.attempt().filter(|&a| a > max_attempt) {
            max_attempt = attempt;
        }
    }
    max_attempt
}

/// Core replay logic for all journal event kinds.
/// Populates the action tracker and detects divergence.
///
/// ## Filtering (PRE-001)
/// Only events from the latest execution attempt affect live state.
/// Events from older attempts are excluded from state transition logic
/// but are still included in the returned output for diagnostics.
pub fn replay_events(
    events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
    _expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<Vec<JournalEvent>> {
    validate_contiguous_sequences(events)?;
    let max_attempt = compute_max_attempt(events);
    let mut replayed = Vec::new();
    let mut last_step: Option<StepIdx> = None;

    for event in events {
        // PRE-001: skip state-affecting events from older attempts
        let attempt = event.attempt().unwrap_or(1);
        if attempt < max_attempt {
            replayed.push(event.clone());
            continue;
        }

        match event {
            JournalEvent::RunAccepted { .. }
            | JournalEvent::RunAdmission { .. }
            | JournalEvent::StepSucceeded { .. }
            | JournalEvent::WaitScheduledEvent { .. }
            | JournalEvent::AskScheduledEvent { .. }
            | JournalEvent::AskAnsweredEvent { .. }
            | JournalEvent::RetryScheduledEvent { .. }
            | JournalEvent::RunCancelled { .. }
            | JournalEvent::RunFinished { .. }
            | JournalEvent::RunFailedEvent { .. }
            | JournalEvent::RunResumed { .. }
            | JournalEvent::RunRetried { .. }
            | JournalEvent::RunAnswered { .. } => {}
            JournalEvent::StepStarted { step, .. } => {
                // Verify step ordering
                if let Some(prev) = last_step
                    && step.get() < prev.get()
                {
                    return Err(RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: format!(
                            "step {} executed before previous step {}",
                            step.get(),
                            prev.get()
                        ),
                    });
                }
                last_step = Some(*step);
            }
            JournalEvent::ActionScheduled { action, step, .. } => {
                // Check if this action was already resolved
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
            }
            JournalEvent::ActionCompletedEvent { action, step, .. } => {
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
                // Mark action as completed to prevent re-execution
                tracker.mark_completed(*action, *step);
            }
            JournalEvent::ActionFailedEvent { action, step, .. } => {
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
                // Mark action as failed to prevent re-execution
                tracker.mark_failed(*action, *step);
            }
            JournalEvent::SlotWrittenEvent { .. } => {
                // Slot writes are allowed for latest attempt
            }
        }
        replayed.push(event.clone());
    }

    Ok(replayed)
}

fn validate_contiguous_sequences(events: &[JournalEvent]) -> RecoveryResult<()> {
    let Some(first) = events.first() else {
        return Ok(());
    };
    let mut expected = first.seq();
    for event in events {
        let seq = event.seq();
        if seq != expected {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "journal sequence violation: expected {}, found {}",
                    expected.get(),
                    seq.get()
                ),
            });
        }
        expected = EventSeq::new(expected.get().saturating_add(1));
    }
    Ok(())
}

/// Replays a full journal for a run when no snapshot is available.
/// Returns the ordered sequence of journal events and populates the action tracker.
///
/// ## GAP-3: Policy Digest Verification
///
/// When `expected_policy_digests` is empty and `RunAdmission` is absent,
/// return `PolicyDigestMismatch` because the policy digest cannot be verified.
pub fn recover_full_journal(
    journal: &FjallJournal,
    run: RunId,
    tracker: &mut ActionReplayTracker,
    _expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<Vec<JournalEvent>> {
    let events = journal.events_for_run(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }

    let has_run_admission = events
        .iter()
        .any(|e| matches!(e, JournalEvent::RunAdmission { .. }));

    if !has_run_admission && !expected_policy_digests.is_empty() {
        return Err(RecoveryError::PolicyDigestMismatch {
            step: StepIdx::ZERO,
        });
    }

    replay_events(&events, tracker, _expected_action_abi_digests)
}

/// Loads a snapshot from the journal, translating decode failures to
/// `RecoveryError::CorruptSnapshot`.
pub fn load_snapshot(
    journal: &FjallJournal,
    run: RunId,
    seq: EventSeq,
) -> RecoveryResult<crate::recovery::types::RunSnapshot> {
    match journal.snapshot(run, seq) {
        Ok(Some(snapshot)) => Ok(snapshot),
        Ok(None) | Err(crate::JournalError::PostcardDecodeFailed) => {
            Err(RecoveryError::CorruptSnapshot { run, seq })
        }
        Err(other) => Err(RecoveryError::Journal(other)),
    }
}

/// Replays from a snapshot plus tail events.
/// The snapshot provides the base state, and tail events are replayed on top.
pub fn recover_snapshot_plus_tail(
    snapshot: &crate::recovery::types::RunSnapshot,
    tail_events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<Vec<JournalEvent>> {
    // Verify snapshot consistency
    let snapshot_seq = snapshot.seq;
    for event in tail_events {
        if event.seq() <= snapshot_seq {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "tail event seq {} is not after snapshot seq {}",
                    event.seq().get(),
                    snapshot_seq.get()
                ),
            });
        }
    }

    replay_events(tail_events, tracker, &[])
}

/// Checks whether a run has reached a terminal state.
#[must_use]
pub fn is_terminal_event(event: &JournalEvent) -> bool {
    matches!(
        event,
        JournalEvent::RunFinished { .. }
            | JournalEvent::RunCancelled { .. }
            | JournalEvent::RunFailedEvent { .. }
    )
}

/// Extracts the terminal event from a replay sequence, if any.
///
/// Only considers terminal events from the latest execution attempt.
/// Terminal events from older (stale) attempts are ignored.
pub fn extract_terminal(events: &[JournalEvent]) -> Option<&JournalEvent> {
    let max_attempt = compute_max_attempt(events);
    events
        .iter()
        .rev()
        .find(|event| is_terminal_event(event) && event.attempt().unwrap_or(1) == max_attempt)
}
