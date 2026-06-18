#![forbid(unsafe_code)]
//! Event replay and tail application for RunFrame hydration.
//!
//! Provides:
//! - `apply_tail_events`: applies journal events to a mutable RunFrame
//! - `compute_parallel_in_flight`: computes peak parallel in-flight from events
//! - `SlotTaintReadObservation` / `SlotTaintResolution` / `resolve_slot_taint_read`
//!
//! These are the core replay primitives: applying deterministic state
//! transitions from journal events onto a live RunFrame.

use crate::DurableActionOutcome;
use crate::JournalEvent;
use crate::recovery::{
    ActionReplayTracker, RecoveryError, RecoveryResult, types::ActionReplayEffect,
};
use vb_core::SlotIdx;
use vb_core::{ActionTicket, RunFrame, RunId, SlotValue, StepIdx, Taint};

// ============================================================================
// Slot taint resolution (pure, tail-event-adjacent)
// ============================================================================

/// Copy-only observation of `RunFrame::read_taint` for fail-closed replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotTaintReadObservation {
    /// Existing taint was read successfully.
    Existing(Taint),
    /// The slot is not initialized, so Clean is the only allowed default.
    Uninitialized,
    /// The taint read failed for any other reason and must fail closed.
    Failed,
}

/// Copy-only taint resolution decision for slot replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotTaintResolution {
    /// Continue with the selected taint value.
    Use(Taint),
    /// Abort replay instead of downgrading taint to Clean.
    FailClosed,
}

/// Resolves a taint read without allocating or mutating the frame.
#[must_use]
pub(crate) const fn resolve_slot_taint_read(
    observation: SlotTaintReadObservation,
) -> SlotTaintResolution {
    match observation {
        SlotTaintReadObservation::Existing(taint) => SlotTaintResolution::Use(taint),
        SlotTaintReadObservation::Uninitialized => SlotTaintResolution::Use(vb_core::Taint::Clean),
        SlotTaintReadObservation::Failed => SlotTaintResolution::FailClosed,
    }
}

fn observe_slot_taint_read(result: Result<Taint, vb_core::CoreError>) -> SlotTaintReadObservation {
    match result {
        Ok(taint) => SlotTaintReadObservation::Existing(taint),
        Err(vb_core::CoreError::SlotUninitialized { .. }) => {
            SlotTaintReadObservation::Uninitialized
        }
        Err(_) => SlotTaintReadObservation::Failed,
    }
}

// ============================================================================
// Action envelope helpers (tail-event-adjacent)
// ============================================================================

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

// ============================================================================
// Tail event application
// ============================================================================

/// Applies tail journal events to a mutable RunFrame, tracking action resolution.
///
/// Returns the count of state-affecting events applied.
pub(super) fn apply_tail_events(
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
                super::action_digest::verify_action_ticket_event(*run, *ticket)?;
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
                let verified_digest = super::action_digest::verified_action_envelope_digest(
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
            JournalEvent::SlotWrittenEvent { slot, value, .. } => {
                if let Some(bytes) = value {
                    let slot_value = postcard::from_bytes(bytes).map_err(|_| {
                        RecoveryError::ReplayDivergence {
                            step: StepIdx::ZERO,
                            detail: format!("slot value decode failed for slot {:?}", slot),
                        }
                    })?;
                    let taint = match resolve_slot_taint_read(observe_slot_taint_read(
                        frame.read_taint(*slot),
                    )) {
                        SlotTaintResolution::Use(taint) => taint,
                        SlotTaintResolution::FailClosed => {
                            return Err(RecoveryError::SlotTaintReadFailed { slot: *slot });
                        }
                    };
                    frame
                        .write_slot_with_taint(*slot, slot_value, taint)
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

// ============================================================================
// Parallel in-flight peak computation
// ============================================================================

/// Computes parallel in-flight counters from action events without mutating step states.
///
/// This is used by `hydrate_run_frame_from_events` after the seed has already
/// applied step states. It only tracks action scheduling/completion for parallel
/// counters.
///
/// Returns the peak parallel in-flight count observed.
pub(super) fn compute_parallel_in_flight(
    frame: &mut RunFrame,
    events: &[JournalEvent],
) -> RecoveryResult<u16> {
    let mut tracker = ActionReplayTracker::new();
    let mut peak: u16 = 0;

    for event in events {
        match event {
            JournalEvent::ActionScheduled { action, step, .. } => {
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
                frame
                    .add_parallel_in_flight(1)
                    .map_err(|_| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "parallel_in_flight overflow".to_owned(),
                    })?;
                if frame.parallel_in_flight() > peak {
                    peak = frame.parallel_in_flight();
                }
            }
            JournalEvent::ActionScheduledTicket {
                run,
                ticket,
                input,
                output,
                ..
            } => {
                super::action_digest::verify_action_ticket_event(*run, *ticket)?;
                let effect = tracker.mark_scheduled_ticket_effect(*ticket, *input, *output)?;
                if effect == ActionReplayEffect::Duplicate {
                    continue;
                }
                frame
                    .add_parallel_in_flight(1)
                    .map_err(|_| RecoveryError::ReplayDivergence {
                        step: ticket.step,
                        detail: "parallel_in_flight overflow".to_owned(),
                    })?;
                if frame.parallel_in_flight() > peak {
                    peak = frame.parallel_in_flight();
                }
            }
            JournalEvent::ActionCompletedEvent { action, step, .. } => {
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
                tracker.mark_completed(*action, *step);
                frame
                    .sub_parallel_in_flight(1)
                    .map_err(|_| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "parallel_in_flight underflow".to_owned(),
                    })?;
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
                let verified_digest = super::action_digest::verified_action_envelope_digest(
                    *run,
                    *ticket,
                    *outcome,
                    value,
                    *encoded_len,
                    *value_digest,
                )?;
                tracker.require_scheduled_ticket(*ticket, *output)?;
                let effect = tracker.mark_completed_envelope_effect(
                    *ticket,
                    *output,
                    *encoded_len,
                    *taint,
                    verified_digest,
                )?;
                if effect == ActionReplayEffect::Apply {
                    frame.sub_parallel_in_flight(1).map_err(|_| {
                        RecoveryError::ReplayDivergence {
                            step: ticket.step,
                            detail: "parallel_in_flight underflow".to_owned(),
                        }
                    })?;
                }
            }
            JournalEvent::ActionFailedEvent { action, step, .. } => {
                if tracker.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
                tracker.mark_failed(*action, *step);
                frame
                    .sub_parallel_in_flight(1)
                    .map_err(|_| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "parallel_in_flight underflow".to_owned(),
                    })?;
            }
            _ => {}
        }
    }

    Ok(peak)
}
