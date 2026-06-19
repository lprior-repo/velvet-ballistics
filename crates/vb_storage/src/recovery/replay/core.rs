#![forbid(unsafe_code)]
//! Core replay logic for journal event kinds.
//!
//! Provides:
//! - `replay_events`: Event replay with divergence detection
//! - `replay_events_with_schedule_requirement`: Replay with schedule enforcement
//! - `validate_contiguous_sequences`: Sequence gap detection

use super::action_abi::validate_action_abi_expectations;
use super::admission::verify_run_admission_evidence;
use super::attempt::{
    replay_attempt_is_current, replay_attempt_is_stale, replay_event_has_state_effect,
    replay_event_is_stale_state_effect, replay_step_order_diverges,
};
use crate::recovery::action_digest::{verified_action_envelope_digest, verify_action_ticket_event};
use crate::recovery::{ActionReplayTracker, RecoveryError, RecoveryResult};
use crate::{EventSeq, JournalEvent};
use vb_core::{ActionId, StepIdx, WorkflowDigest};

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

pub(super) fn replay_events_with_schedule_requirement(
    events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
    require_schedule: bool,
) -> RecoveryResult<Vec<JournalEvent>> {
    validate_contiguous_sequences(events)?;
    let max_attempt = super::attempt::compute_max_attempt(events);
    let mut replayed = Vec::new();
    let mut last_step: Option<StepIdx> = None;

    for event in events {
        // PRE-001: skip state-affecting events from older attempts
        if super::attempt::replay_attempt_is_stale(event.attempt(), max_attempt) {
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
