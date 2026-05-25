#![forbid(unsafe_code)]
//! RunFrame hydration support functions.
//!
//! Internal helpers for slot decoding, dimension derivation, event application,
//! and parallel in-flight tracking. Not part of the public API.

use crate::JournalEvent;
use crate::recovery::types::{
    ActionReplayEffect, ActionReplayTracker, RecoveryError, RecoveryResult, RunSnapshot,
};
use vb_core::RunId;

/// Copy-only observation of `RunFrame::read_taint` for fail-closed replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotTaintReadObservation {
    /// Existing taint was read successfully.
    Existing(vb_core::Taint),
    /// The slot is not initialized, so Clean is the only allowed default.
    Uninitialized,
    /// The taint read failed for any other reason and must fail closed.
    Failed,
}

/// Copy-only taint resolution decision for slot replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotTaintResolution {
    /// Continue with the selected taint value.
    Use(vb_core::Taint),
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

fn observe_slot_taint_read(
    result: Result<vb_core::Taint, vb_core::CoreError>,
) -> SlotTaintReadObservation {
    match result {
        Ok(taint) => SlotTaintReadObservation::Existing(taint),
        Err(vb_core::CoreError::SlotUninitialized { .. }) => {
            SlotTaintReadObservation::Uninitialized
        }
        Err(_) => SlotTaintReadObservation::Failed,
    }
}

pub(crate) fn verified_action_envelope_digest(
    ticket: vb_core::ActionTicket,
    value: &[u8],
    encoded_len: u32,
    expected: [u8; 32],
) -> RecoveryResult<[u8; 32]> {
    let actual_len = u32::try_from(value.len()).map_err(|_| RecoveryError::ReplayDivergence {
        step: ticket.step,
        detail: String::from("action completion value length exceeds u32"),
    })?;
    if actual_len != encoded_len {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action completion encoded length mismatch"),
        });
    }

    let found = *blake3::hash(value).as_bytes();
    if found == expected {
        Ok(expected)
    } else {
        Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action completion value digest mismatch"),
        })
    }
}

fn decode_action_envelope_slot(
    ticket: vb_core::ActionTicket,
    output: vb_core::SlotIdx,
    value: &[u8],
) -> RecoveryResult<vb_core::SlotValue> {
    postcard::from_bytes(value).map_err(|_| RecoveryError::ReplayDivergence {
        step: ticket.step,
        detail: format!("slot value decode failed for slot {:?}", output),
    })
}

/// Decodes snapshot slot/taint bytes into recovered slot entries.
///
/// Expects postcard-encoded `Vec<(SlotIdx, SlotValue, Taint)>` in the slots field,
/// and the same format in the taint field (used for validation/merge).
pub(super) fn decode_snapshot_slots(
    slots_bytes: &[u8],
    taint_bytes: &[u8],
    run: RunId,
) -> RecoveryResult<Vec<crate::recovery::types::RecoveredSlotEntry>> {
    if slots_bytes.is_empty() && taint_bytes.is_empty() {
        return Ok(Vec::new());
    }

    let slots: Vec<(vb_core::SlotIdx, vb_core::SlotValue, vb_core::Taint)> =
        postcard::from_bytes(slots_bytes).map_err(|_| RecoveryError::CorruptSnapshot {
            run,
            seq: crate::EventSeq::new(0),
        })?;

    let taint: Vec<(vb_core::SlotIdx, vb_core::SlotValue, vb_core::Taint)> =
        postcard::from_bytes(taint_bytes).map_err(|_| RecoveryError::CorruptSnapshot {
            run,
            seq: crate::EventSeq::new(0),
        })?;

    // Merge slots and taint, preferring explicit taint from the taint vector
    let mut entries = Vec::new();
    for (slot, value, default_taint) in slots {
        let explicit_taint = taint
            .iter()
            .find_map(|(t_slot, _, t_taint)| {
                if *t_slot == slot {
                    Some(*t_taint)
                } else {
                    None
                }
            })
            .unwrap_or(default_taint);
        entries.push(crate::recovery::types::RecoveredSlotEntry {
            slot,
            value,
            taint: explicit_taint,
        });
    }

    Ok(entries)
}

/// Derives step_count, slot_count, and first_step from snapshot + tail events.
///
/// Accepts pre-decoded snapshot slots to avoid double-decoding the snapshot bytes.
pub(super) fn derive_dimensions_from_snapshot_and_tail(
    _snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run: RunId,
    snapshot_slots: &[crate::recovery::types::RecoveredSlotEntry],
) -> RecoveryResult<(u16, u16, vb_core::StepIdx)> {
    let mut max_step: Option<vb_core::StepIdx> = None;
    let mut min_step: Option<vb_core::StepIdx> = None;
    let mut max_slot: Option<vb_core::SlotIdx> = None;

    // Use pre-decoded snapshot slots to find max slot index
    if !snapshot_slots.is_empty() {
        for entry in snapshot_slots {
            max_slot = Some(max_slot.map_or(entry.slot, |s| s.max(entry.slot)));
        }
    }

    // Scan tail events for max step/slot
    for event in tail_events {
        match event {
            JournalEvent::StepStarted { step, .. }
            | JournalEvent::StepSucceeded { step, .. }
            | JournalEvent::ActionScheduled { step, .. }
            | JournalEvent::ActionCompletedEvent { step, .. }
            | JournalEvent::ActionFailedEvent { step, .. }
            | JournalEvent::WaitScheduledEvent { step, .. }
            | JournalEvent::AskScheduledEvent { step, .. }
            | JournalEvent::RetryScheduledEvent { step, .. } => {
                max_step = Some(max_step.map_or(*step, |s| s.max(*step)));
                min_step = Some(min_step.map_or(*step, |s| s.min(*step)));
            }
            JournalEvent::ActionScheduledTicket { ticket, .. } => {
                max_step = Some(max_step.map_or(ticket.step, |s| s.max(ticket.step)));
                min_step = Some(min_step.map_or(ticket.step, |s| s.min(ticket.step)));
            }
            JournalEvent::ActionCompletedEnvelope { ticket, output, .. } => {
                max_step = Some(max_step.map_or(ticket.step, |s| s.max(ticket.step)));
                min_step = Some(min_step.map_or(ticket.step, |s| s.min(ticket.step)));
                max_slot = Some(max_slot.map_or(*output, |s| s.max(*output)));
            }
            JournalEvent::SlotWrittenEvent { slot, .. }
            | JournalEvent::RunFinished { result: slot, .. } => {
                max_slot = Some(max_slot.map_or(*slot, |s| s.max(*slot)));
            }
            _ => {}
        }
    }

    let step_count = max_step
        .map(|s| {
            s.get()
                .checked_add(1)
                .ok_or(RecoveryError::FrameDimensionOverflow { run })
        })
        .unwrap_or(Ok(0))?;

    let slot_count = max_slot
        .map(|s| {
            s.get()
                .checked_add(1)
                .ok_or(RecoveryError::FrameDimensionOverflow { run })
        })
        .unwrap_or(Ok(0))?;

    let first_step = min_step.unwrap_or(vb_core::StepIdx::ZERO);

    Ok((step_count, slot_count, first_step))
}

/// Applies tail journal events to a mutable RunFrame, tracking action resolution.
///
/// Returns the count of state-affecting events applied.
pub(super) fn apply_tail_events(
    frame: &mut vb_core::RunFrame,
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
            JournalEvent::ActionScheduledTicket { ticket, .. } => {
                if tracker.is_resolved(ticket.action, ticket.step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: ticket.action,
                        step: ticket.step,
                    });
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
                frame
                    .sub_parallel_in_flight(1)
                    .map_err(|_e| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "parallel_in_flight underflow".to_owned(),
                    })?;
                executed = executed.saturating_add(1);
            }
            JournalEvent::ActionCompletedEnvelope {
                ticket,
                output,
                value,
                encoded_len,
                taint,
                value_digest,
                ..
            } => {
                let verified_digest =
                    verified_action_envelope_digest(*ticket, value, *encoded_len, *value_digest)?;
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
                frame
                    .sub_parallel_in_flight(1)
                    .map_err(|_e| RecoveryError::ReplayDivergence {
                        step: ticket.step,
                        detail: "parallel_in_flight underflow".to_owned(),
                    })?;
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
                frame
                    .sub_parallel_in_flight(1)
                    .map_err(|_e| RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: "parallel_in_flight underflow".to_owned(),
                    })?;
                executed = executed.saturating_add(1);
            }
            JournalEvent::SlotWrittenEvent { slot, value, .. } => {
                if let Some(bytes) = value {
                    let slot_value = postcard::from_bytes(bytes).map_err(|_| {
                        RecoveryError::ReplayDivergence {
                            step: vb_core::StepIdx::ZERO,
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
                            step: vb_core::StepIdx::ZERO,
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

/// Computes parallel in-flight counters from action events without mutating step states.
///
/// This is used by `hydrate_run_frame_from_events` after the seed has already
/// applied step states. It only tracks action scheduling/completion for parallel counters.
///
/// Returns the peak parallel in-flight count observed.
pub(super) fn compute_parallel_in_flight(
    frame: &mut vb_core::RunFrame,
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
            JournalEvent::ActionScheduledTicket { ticket, .. } => {
                if tracker.is_resolved(ticket.action, ticket.step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: ticket.action,
                        step: ticket.step,
                    });
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
                ticket,
                output,
                value,
                encoded_len,
                taint,
                value_digest,
                ..
            } => {
                let verified_digest =
                    verified_action_envelope_digest(*ticket, value, *encoded_len, *value_digest)?;
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
