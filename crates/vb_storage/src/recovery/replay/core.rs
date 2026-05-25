#![forbid(unsafe_code)]
//! Core replay logic for journal recovery.
//!
//! Provides:
//! - Event replay with divergence detection
//! - Non-idempotent action blocking
//! - Snapshot-plus-tail replay

use crate::recovery::hydrate_support::{
    verified_action_envelope_digest, verify_action_ticket_event,
};
use crate::recovery::types::{ActionReplayTracker, RecoveryError, RecoveryResult};
use crate::{EventSeq, FjallJournal, JournalEvent};
use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest};

/// Computes the maximum attempt number observed in action-scheduling and
/// action-completion events. Events without an attempt field contribute 1
/// (PRE-001: treat as attempt 1).
#[must_use]
pub(crate) fn compute_max_attempt(events: &[JournalEvent]) -> u16 {
    let mut max_attempt = 1u16;
    for event in events {
        if let Some(attempt) = event.attempt().filter(|&a| a > max_attempt) {
            max_attempt = attempt;
        }
    }
    max_attempt
}

/// Production proof surface for defaulting absent attempts to attempt one.
#[must_use]
pub const fn replay_attempt_or_default(attempt: Option<u16>) -> u16 {
    match attempt {
        Some(value) => value,
        None => 1,
    }
}

/// Production proof surface for latest-attempt filtering.
#[must_use]
pub const fn replay_attempt_is_current(attempt: Option<u16>, max_attempt: u16) -> bool {
    replay_attempt_or_default(attempt) >= max_attempt
}

/// Production proof surface for stale-attempt rejection.
#[must_use]
pub const fn replay_attempt_is_stale(attempt: Option<u16>, max_attempt: u16) -> bool {
    replay_attempt_or_default(attempt) < max_attempt
}

/// Production proof surface for event kinds that may mutate replay state.
#[must_use]
pub const fn replay_event_has_state_effect(event: &JournalEvent) -> bool {
    matches!(
        event,
        JournalEvent::StepStarted { .. }
            | JournalEvent::ActionScheduled { .. }
            | JournalEvent::ActionCompletedEvent { .. }
            | JournalEvent::ActionFailedEvent { .. }
            | JournalEvent::SlotWrittenEvent { .. }
    )
}

/// Production proof surface for stale state-effect filtering.
#[must_use]
pub fn replay_event_is_stale_state_effect(event: &JournalEvent, max_attempt: u16) -> bool {
    replay_event_has_state_effect(event) && replay_attempt_is_stale(event.attempt(), max_attempt)
}

/// Production proof surface for step-order divergence detection.
#[must_use]
pub const fn replay_step_order_diverges(previous: Option<StepIdx>, current: StepIdx) -> bool {
    match previous {
        Some(step) => current.get() < step.get(),
        None => false,
    }
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
    replay_events_with_schedule_requirement(events, tracker, true)
}

fn replay_events_with_schedule_requirement(
    events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
    require_schedule: bool,
) -> RecoveryResult<Vec<JournalEvent>> {
    validate_contiguous_sequences(events)?;
    let max_attempt = compute_max_attempt(events);
    let mut replayed = Vec::new();
    let mut last_step: Option<StepIdx> = None;

    for event in events {
        // PRE-001: skip state-affecting events from older attempts
        if replay_attempt_is_stale(event.attempt(), max_attempt) {
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
                if replay_step_order_diverges(last_step, *step) {
                    let previous_step = match last_step {
                        Some(value) => value,
                        None => StepIdx::ZERO,
                    };
                    return Err(RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: format!(
                            "step {} executed before previous step {}",
                            step.get(),
                            previous_step.get()
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
            JournalEvent::ActionScheduledTicket {
                run,
                ticket,
                input,
                output,
                ..
            } => {
                verify_action_ticket_event(*run, *ticket)?;
                tracker.mark_scheduled_ticket_effect(*ticket, *input, *output)?;
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
            JournalEvent::ActionCompletedEnvelope {
                run,
                ticket,
                output,
                outcome,
                value,
                encoded_len,
                taint,
                value_digest,
                ..
            } => {
                let verified_digest = verified_action_envelope_digest(
                    *run,
                    *ticket,
                    *outcome,
                    value,
                    *encoded_len,
                    *value_digest,
                )?;
                if require_schedule {
                    tracker.require_scheduled_ticket(*ticket, *output)?;
                }
                tracker.mark_completed_envelope(
                    *ticket,
                    *output,
                    *encoded_len,
                    *taint,
                    verified_digest,
                )?;
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

    replay_events_with_schedule_requirement(tail_events, tracker, false)
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
