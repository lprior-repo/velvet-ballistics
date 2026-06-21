#![forbid(unsafe_code)]
//! Tail event application for RunFrame hydration.

use crate::JournalEvent;
use crate::recovery::{
    ActionReplayTracker, RecoveryError, RecoveryResult, types::ActionReplayEffect,
};
use vb_core::SlotIdx;
use vb_core::{ActionTicket, RunFrame, SlotValue, StepIdx};

fn decode_action_envelope_slot(
    ticket: ActionTicket,
    output: SlotIdx,
    value: &[u8],
) -> RecoveryResult<SlotValue> {
    postcard::from_bytes(value).map_err(|_| RecoveryError::ReplayDivergence {
        step: ticket.step,
        detail: format!("slot value decode failed for slot {:?}", output),
    })
}

fn sub_tail_parallel_in_flight(frame: &mut RunFrame, step: StepIdx) -> RecoveryResult<()> {
    if frame.parallel_in_flight() == 0 {
        return Ok(());
    }
    frame
        .sub_parallel_in_flight(1)
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step,
            detail: "parallel_in_flight underflow".to_owned(),
        })
}

/// Applies tail journal events to a mutable RunFrame, tracking action resolution.
///
/// Returns the count of state-affecting events applied.
pub(crate) fn apply_tail_events(
    frame: &mut RunFrame,
    tail_events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<u64> {
    // Reset max_parallel_in_flight to 0 so add_parallel_in_flight correctly
    // tracks the peak observed during replay (matching the full-journal path).
    frame.set_max_parallel_in_flight(0);
    let mut executed = 0u64;
    for event in tail_events {
        match event {
            JournalEvent::StepStarted { step, .. } => {
                frame
                    .mark_running(*step)
                    .map_err(|_e| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "mark_running failed".to_owned(),
                    })?;
                executed = executed.saturating_add(1);
            }
            JournalEvent::StepSucceeded { step, .. } => {
                frame
                    .mark_succeeded(*step)
                    .map_err(|_e| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "mark_succeeded failed".to_owned(),
                    })?;
                executed = executed.saturating_add(1);
            }
            JournalEvent::ActionScheduled { action, step, .. } => {
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
                frame
                    .add_parallel_in_flight(1)
                    .map_err(|_e| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "parallel_in_flight overflow".to_owned(),
                    })?;
                executed = executed.saturating_add(1);
            }
            JournalEvent::ActionScheduledTicket {
                run,
                ticket,
                input,
                output,
                ..
            } => {
                crate::recovery::action_digest::verify_action_ticket_event(*run, *ticket)?;
                let effect = tracker.mark_scheduled_ticket_effect(*ticket, *input, *output)?;
                if effect == ActionReplayEffect::Duplicate {
                    continue;
                }
                frame
                    .add_parallel_in_flight(1)
                    .map_err(|_e| RecoveryError::ReplayDivergence {
                        step: ticket.step,
                        detail: "parallel_in_flight overflow".to_owned(),
                    })?;
                executed = executed.saturating_add(1);
            }
            JournalEvent::ActionCompletedEvent { action, step, .. } => {
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
                tracker.mark_completed(*action, *step);
                sub_tail_parallel_in_flight(frame, *step)?;
                executed = executed.saturating_add(1);
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
                let verified_digest =
                    crate::recovery::action_digest::verified_action_envelope_digest(
                        *run,
                        *ticket,
                        *outcome,
                        value,
                        *encoded_len,
                        *value_digest,
                    )?;
                let effect = tracker.mark_completed_envelope_effect(
                    *ticket,
                    *output,
                    *encoded_len,
                    *taint,
                    verified_digest,
                )?;
                if effect == ActionReplayEffect::Duplicate {
                    continue;
                }
                let slot_value = decode_action_envelope_slot(*ticket, *output, value)?;
                frame
                    .write_slot_with_taint(*output, slot_value, *taint)
                    .map_err(|_| RecoveryError::ReplayDivergence {
                        step: ticket.step,
                        detail: "slot write out of bounds".to_owned(),
                    })?;
                frame
                    .mark_succeeded(ticket.step)
                    .map_err(|_| RecoveryError::ReplayDivergence {
                        step: ticket.step,
                        detail: "mark_succeeded failed".to_owned(),
                    })?;
                sub_tail_parallel_in_flight(frame, ticket.step)?;
                executed = executed.saturating_add(1);
            }
            JournalEvent::ActionFailedEvent { action, step, .. } => {
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
                tracker.mark_failed(*action, *step);
                sub_tail_parallel_in_flight(frame, *step)?;
                executed = executed.saturating_add(1);
            }
            JournalEvent::SlotWrittenEvent { slot, value, extra, .. } => {
                if let Some(bytes) = value {
                    let slot_value = postcard::from_bytes::<SlotValue>(bytes).map_err(|_| {
                        RecoveryError::ReplayDivergence {
                            step: StepIdx::ZERO,
                            detail: format!("slot value decode failed for slot {:?}", slot),
                        }
                    })?;
                    // SR-003: decode the slot taint from the persisted envelope
                    // (when present) instead of inheriting whatever happens to be
                    // in the frame. This restores parity with the accumulator path
                    // and ensures Secret-derived slot writes are not silently
                    // downgraded to Clean during events-only hydration.
                    let recovered = crate::recovery::replay::summary::slots::taint::recovered_slot_taint(
                        *slot,
                        slot_value,
                        extra.as_ref(),
                    )?;
                    frame
                        .write_slot_with_taint(*slot, slot_value, recovered.taint)
                        .map_err(|_| RecoveryError::ReplayDivergence {
                            step: StepIdx::ZERO,
                            detail: "slot write out of bounds".to_owned(),
                        })?;
                }
                executed = executed.saturating_add(1);
            }
            JournalEvent::WaitScheduledEvent { step, .. } => {
                // A wait can be scheduled before the step is marked Running.
                // Transition through Running first to satisfy state machine rules.
                let current_state =
                    frame
                        .step_state(*step)
                        .map_err(|_| RecoveryError::ReplayDivergence {
                            step: *step,
                            detail: "step_state read failed".to_owned(),
                        })?;
                if current_state == vb_core::StepState::Pending {
                    frame
                        .mark_running(*step)
                        .map_err(|_e| RecoveryError::ReplayDivergence {
                            step: *step,
                            detail: "mark_running before waiting failed".to_owned(),
                        })?;
                }
                frame
                    .mark_waiting(*step)
                    .map_err(|_e| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "mark_waiting failed".to_owned(),
                    })?;
                executed = executed.saturating_add(1);
            }
            JournalEvent::AskScheduledEvent { step, .. } => {
                let current_state =
                    frame
                        .step_state(*step)
                        .map_err(|_| RecoveryError::ReplayDivergence {
                            step: *step,
                            detail: "step_state read failed".to_owned(),
                        })?;
                if current_state == vb_core::StepState::Pending {
                    frame
                        .mark_running(*step)
                        .map_err(|_e| RecoveryError::ReplayDivergence {
                            step: *step,
                            detail: "mark_running before asking failed".to_owned(),
                        })?;
                }
                frame
                    .mark_asking(*step)
                    .map_err(|_e| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "mark_asking failed".to_owned(),
                    })?;
                executed = executed.saturating_add(1);
            }
            _ => {}
        }
    }
    Ok(executed)
}
