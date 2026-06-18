#![forbid(unsafe_code)]
//! Core replay logic for journal recovery.
//!
//! Provides:
//! - Event replay with divergence detection
//! - Non-idempotent action blocking
//! - Snapshot-plus-tail replay

use super::action_abi::validate_action_abi_expectations;
use super::admission::verify_run_admission_evidence;
use super::attempt::compute_max_attempt;
pub use super::attempt::{
    replay_attempt_is_current, replay_attempt_is_stale, replay_attempt_or_default,
    replay_event_has_state_effect, replay_event_is_stale_state_effect, replay_step_order_diverges,
};
use crate::records::RecoveryStampRecord;
use crate::recovery::hydrate_support::{
    verified_action_envelope_digest, verify_action_ticket_event,
};
use crate::recovery::{ActionReplayTracker, RecoveryError, RecoveryResult};
use crate::{EventSeq, FjallJournal, JournalEvent};
use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest};

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
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<Vec<JournalEvent>> {
    validate_action_abi_expectations(events, expected_action_abi_digests)?;
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
            | JournalEvent::RunKilled { .. }
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
    events.windows(2).try_for_each(|pair| {
        let [previous_event, next_event] = pair else {
            return Ok(());
        };
        let previous = previous_event.seq();
        let found = next_event.seq();
        let Some(expected) = previous.get().checked_add(1).map(EventSeq::new) else {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "journal sequence overflow after {} before {}",
                    previous.get(),
                    found.get()
                ),
            });
        };

        if found != expected {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "journal sequence violation: expected {}, found {}",
                    expected.get(),
                    found.get()
                ),
            });
        }
        Ok(())
    })
}

/// Replays a full journal for a run when no snapshot is available.
/// Returns the ordered sequence of journal events and populates the action tracker.
///
/// ## GAP-3: Policy Digest Verification
///
/// Missing durable `RunAdmission` evidence fails closed instead of replaying
/// without proof or fabricating a digest value.
///
/// ## Recovery stamp integration (vb-k6iwh-r)
///
/// Before replay, the recovery stamp keyspace is consulted at `(run, last_seq)`.
/// A present stamp indicates a prior recovery attempt has already replayed
/// the journal up to `last_seq`; replay is still run (to refresh the action
/// tracker for the caller) but no new stamp is written, preserving the
/// existing marker. The skip-replay semantic — using the stamp to short-circuit
/// replay when the journal tail is unchanged — is delegated to a follow-up
/// bead that will compare the stamp's `last_seq` against the journal tail.
///
/// After a successful replay with no prior stamp, a fresh `RecoveryStampRecord`
/// is persisted at `(run, last_seq)` so a subsequent recovery invocation can
/// detect that the replay for this run has already been performed.
pub fn recover_full_journal(
    journal: &FjallJournal,
    run: RunId,
    tracker: &mut ActionReplayTracker,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<Vec<JournalEvent>> {
    let events = journal.events_for_run(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }

    verify_run_admission_evidence(&events, run, expected_policy_digests)?;

    let last_seq = events.last().map_or(EventSeq::ZERO, JournalEvent::seq);
    let replayed = replay_events(&events, tracker, expected_action_abi_digests)?;

    if journal.get_recovery_stamp(run, last_seq)?.is_none() {
        write_recovery_stamp(journal, run, last_seq)?;
    }
    Ok(replayed)
}

/// Persists a `RecoveryStampRecord` for `(run, last_seq)` after successful replay.
///
/// The millisecond timestamp is derived from the wall clock and narrowed to
/// `u64` with checked arithmetic; a clock that exceeds `u64::MAX` ms (year
/// ~584 billion) saturates to `u64::MAX` so the stamp is always writable.
fn write_recovery_stamp(
    journal: &FjallJournal,
    run: RunId,
    last_seq: EventSeq,
) -> RecoveryResult<()> {
    let written_at_ms = current_unix_millis_saturating();
    let stamp = RecoveryStampRecord {
        run,
        last_seq,
        written_at_ms,
    };
    journal.put_recovery_stamp(run, last_seq, stamp)?;
    Ok(())
}

/// Returns the current wall-clock time in milliseconds since the Unix epoch,
/// saturating to `u64::MAX` on overflow. The conversion is checked, not lossy.
fn current_unix_millis_saturating() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let millis = duration.as_millis();
    if millis > u128::from(u64::MAX) {
        u64::MAX
    } else {
        u64::try_from(millis).unwrap_or(u64::MAX)
    }
}

/// Loads a snapshot from the journal, translating decode failures to
/// `RecoveryError::CorruptSnapshot`.
pub fn load_snapshot(
    journal: &FjallJournal,
    run: RunId,
    seq: EventSeq,
) -> RecoveryResult<crate::recovery::RunSnapshot> {
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
    snapshot: &crate::recovery::RunSnapshot,
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
