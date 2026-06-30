#![forbid(unsafe_code)]
//! RunFrame hydration support functions.
//!
//! Internal helpers for slot decoding, dimension derivation, event application,
//! and parallel in-flight tracking. Not part of the public API.

use std::collections::HashMap;

use crate::recovery::types::{
    ActionReplayEffect, ActionReplayTracker, RecoveryError, RecoveryResult, RunSnapshot,
};
use crate::{DurableActionOutcome, JournalEvent};
use vb_core::{ActionTicket, RunId};

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

pub(crate) fn verify_action_ticket_event(run: RunId, ticket: ActionTicket) -> RecoveryResult<()> {
    if ticket.run != run {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action ticket run mismatch"),
        });
    }
    if ticket.attempt == 0 || ticket.capacity == 0 || ticket.attempt > ticket.capacity {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action ticket attempt bounds invalid"),
        });
    }
    if !vb_core::action::action_ticket_has_valid_key(ticket) {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action ticket idempotency key mismatch"),
        });
    }
    Ok(())
}

pub(crate) fn verified_action_envelope_digest(
    run: RunId,
    ticket: ActionTicket,
    outcome: DurableActionOutcome,
    value: &[u8],
    encoded_len: u32,
    expected: [u8; 32],
) -> RecoveryResult<[u8; 32]> {
    verify_action_ticket_event(run, ticket)?;
    match outcome {
        DurableActionOutcome::Ready => {}
    }
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

fn sub_tail_parallel_in_flight(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
) -> RecoveryResult<()> {
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

    // Merge slots and taint, preferring explicit taint from the taint vector.
    let taint_by_slot: HashMap<vb_core::SlotIdx, vb_core::Taint> = taint
        .into_iter()
        .map(|(slot, _, explicit_taint)| (slot, explicit_taint))
        .collect();
    let mut entries = Vec::with_capacity(slots.len());
    for (slot, value, default_taint) in slots {
        let explicit_taint = match taint_by_slot.get(&slot) {
            Some(taint) => *taint,
            None => default_taint,
        };
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
            | JournalEvent::AskTimedOutEvent { step, .. }
            | JournalEvent::WaitResolvedEvent { step, .. }
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

/// Categorizes the result of dispatching an event to a category helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApplyOutcome {
    /// Helper handled the event and contributed to the executed count.
    Executed,
    /// Helper handled the event but the caller must not increment (Duplicate skip).
    Skipped,
    /// Helper did not recognize the event variant.
    NotApplicable,
}

/// Applies tail journal events to a mutable RunFrame, tracking action resolution.
///
/// Returns the count of state-affecting events applied.
pub(super) fn apply_tail_events(
    frame: &mut vb_core::RunFrame,
    tail_events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<u64> {
    frame.set_max_parallel_in_flight(0);
    let mut executed = 0u64;
    for event in tail_events {
        let outcome = match event {
            JournalEvent::StepStarted { .. } | JournalEvent::StepSucceeded { .. } => {
                apply_step_lifecycle(frame, event)?
            }
            JournalEvent::ActionScheduled { .. } => apply_action_scheduled(frame, event, tracker)?,
            JournalEvent::ActionScheduledTicket { .. } => {
                apply_action_scheduled_ticket(frame, event, tracker)?
            }
            JournalEvent::ActionCompletedEvent { .. } => {
                apply_action_completed_event(frame, event, tracker)?
            }
            JournalEvent::ActionCompletedEnvelope { .. } => {
                apply_action_completed_envelope(frame, event, tracker)?
            }
            JournalEvent::ActionFailedEvent { .. } => apply_action_failed(frame, event, tracker)?,
            JournalEvent::SlotWrittenEvent { .. } => apply_slot_written(frame, event)?,
            JournalEvent::WaitScheduledEvent { .. }
            | JournalEvent::AskScheduledEvent { .. }
            | JournalEvent::AskTimedOutEvent { .. } => apply_signal_event(frame, event)?,
            _ => ApplyOutcome::NotApplicable,
        };
        if outcome == ApplyOutcome::Executed {
            executed = executed.saturating_add(1);
        }
    }
    Ok(executed)
}

fn apply_step_lifecycle(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
) -> RecoveryResult<ApplyOutcome> {
    match event {
        JournalEvent::StepStarted { step, .. } => {
            frame
                .mark_running(*step)
                .map_err(|_e| RecoveryError::ReplayDivergence {
                    step: *step,
                    detail: "mark_running failed".to_owned(),
                })?;
            Ok(ApplyOutcome::Executed)
        }
        JournalEvent::StepSucceeded { step, .. } => {
            frame
                .mark_succeeded(*step)
                .map_err(|_e| RecoveryError::ReplayDivergence {
                    step: *step,
                    detail: "mark_succeeded failed".to_owned(),
                })?;
            Ok(ApplyOutcome::Executed)
        }
        _ => Ok(ApplyOutcome::NotApplicable),
    }
}

fn apply_action_scheduled(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
    tracker: &ActionReplayTracker,
) -> RecoveryResult<ApplyOutcome> {
    let JournalEvent::ActionScheduled { action, step, .. } = event else {
        return Ok(ApplyOutcome::NotApplicable);
    };
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
    Ok(ApplyOutcome::Executed)
}

fn apply_action_scheduled_ticket(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<ApplyOutcome> {
    let JournalEvent::ActionScheduledTicket {
        run,
        ticket,
        input,
        output,
        ..
    } = event
    else {
        return Ok(ApplyOutcome::NotApplicable);
    };
    verify_action_ticket_event(*run, *ticket)?;
    let effect = tracker.mark_scheduled_ticket_effect(*ticket, *input, *output)?;
    if effect == ActionReplayEffect::Duplicate {
        return Ok(ApplyOutcome::Skipped);
    }
    frame
        .add_parallel_in_flight(1)
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: "parallel_in_flight overflow".to_owned(),
        })?;
    Ok(ApplyOutcome::Executed)
}

fn apply_action_completed_event(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<ApplyOutcome> {
    let JournalEvent::ActionCompletedEvent { action, step, .. } = event else {
        return Ok(ApplyOutcome::NotApplicable);
    };
    if tracker.is_resolved(*action, *step) {
        return Err(RecoveryError::NonIdempotentActionBlocked {
            action: *action,
            step: *step,
        });
    }
    tracker.mark_completed(*action, *step);
    sub_tail_parallel_in_flight(frame, *step)?;
    Ok(ApplyOutcome::Executed)
}

fn apply_action_completed_envelope(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<ApplyOutcome> {
    let JournalEvent::ActionCompletedEnvelope {
        run,
        ticket,
        output,
        outcome,
        value,
        encoded_len,
        taint,
        value_digest,
        ..
    } = event
    else {
        return Ok(ApplyOutcome::NotApplicable);
    };
    let verified = verified_action_envelope_digest(
        *run,
        *ticket,
        *outcome,
        value,
        *encoded_len,
        *value_digest,
    )?;
    let effect =
        tracker.mark_completed_envelope_effect(*ticket, *output, *encoded_len, *taint, verified)?;
    if effect == ActionReplayEffect::Duplicate {
        return Ok(ApplyOutcome::Skipped);
    }
    apply_envelope_slot_and_step(frame, *ticket, *output, value, *taint)?;
    Ok(ApplyOutcome::Executed)
}

fn apply_envelope_slot_and_step(
    frame: &mut vb_core::RunFrame,
    ticket: vb_core::ActionTicket,
    output: vb_core::SlotIdx,
    value: &[u8],
    taint: vb_core::Taint,
) -> RecoveryResult<()> {
    let slot_value = decode_action_envelope_slot(ticket, output, value)?;
    frame
        .write_slot_with_taint(output, slot_value, taint)
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
    sub_tail_parallel_in_flight(frame, ticket.step)
}

fn apply_action_failed(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<ApplyOutcome> {
    let JournalEvent::ActionFailedEvent { action, step, .. } = event else {
        return Ok(ApplyOutcome::NotApplicable);
    };
    if tracker.is_resolved(*action, *step) {
        return Err(RecoveryError::NonIdempotentActionBlocked {
            action: *action,
            step: *step,
        });
    }
    tracker.mark_failed(*action, *step);
    sub_tail_parallel_in_flight(frame, *step)?;
    Ok(ApplyOutcome::Executed)
}

fn reject_missing_slot_payload(run: RunId) -> RecoveryError {
    RecoveryError::UnsupportedFrameSeed {
        run,
        reason: String::from("slot_values"),
    }
}

fn decode_slot_value(
    bytes: &[u8],
    run: RunId,
    slot: vb_core::SlotIdx,
) -> RecoveryResult<vb_core::SlotValue> {
    postcard::from_bytes(bytes).map_err(|_| RecoveryError::ReplayDivergence {
        step: vb_core::StepIdx::ZERO,
        detail: format!("slot value decode failed for run {:?} slot {:?}", run, slot),
    })
}

fn apply_slot_written(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
) -> RecoveryResult<ApplyOutcome> {
    let JournalEvent::SlotWrittenEvent {
        run, slot, value, ..
    } = event
    else {
        return Ok(ApplyOutcome::NotApplicable);
    };
    let Some(bytes) = value else {
        return Err(reject_missing_slot_payload(*run));
    };
    let slot_value = decode_slot_value(bytes, *run, *slot)?;
    let taint = resolve_slot_taint_or_fail(frame, *slot)?;
    write_slot_with_replay_divergence(frame, *slot, slot_value, taint)?;
    Ok(ApplyOutcome::Executed)
}

fn write_slot_with_replay_divergence(
    frame: &mut vb_core::RunFrame,
    slot: vb_core::SlotIdx,
    slot_value: vb_core::SlotValue,
    taint: vb_core::Taint,
) -> RecoveryResult<()> {
    frame
        .write_slot_with_taint(slot, slot_value, taint)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: "slot write out of bounds".to_owned(),
        })
}

fn resolve_slot_taint_or_fail(
    frame: &vb_core::RunFrame,
    slot: vb_core::SlotIdx,
) -> RecoveryResult<vb_core::Taint> {
    match resolve_slot_taint_read(observe_slot_taint_read(frame.read_taint(slot))) {
        SlotTaintResolution::Use(taint) => Ok(taint),
        SlotTaintResolution::FailClosed => Err(RecoveryError::SlotTaintReadFailed { slot }),
    }
}

fn apply_signal_event(
    frame: &mut vb_core::RunFrame,
    event: &JournalEvent,
) -> RecoveryResult<ApplyOutcome> {
    match event {
        JournalEvent::WaitScheduledEvent { step, .. } => apply_signal_wait(frame, *step),
        JournalEvent::AskScheduledEvent { step, .. } => apply_signal_ask(frame, *step),
        JournalEvent::AskTimedOutEvent { step, .. } => apply_signal_ask_timeout(frame, *step),
        _ => Ok(ApplyOutcome::NotApplicable),
    }
}

fn apply_signal_wait(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
) -> RecoveryResult<ApplyOutcome> {
    ensure_step_running(frame, step, "waiting")?;
    frame
        .mark_waiting(step)
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step,
            detail: "mark_waiting failed".to_owned(),
        })?;
    Ok(ApplyOutcome::Executed)
}

fn apply_signal_ask(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
) -> RecoveryResult<ApplyOutcome> {
    ensure_step_running(frame, step, "asking")?;
    frame
        .mark_asking(step)
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step,
            detail: "mark_asking failed".to_owned(),
        })?;
    Ok(ApplyOutcome::Executed)
}

fn apply_signal_ask_timeout(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
) -> RecoveryResult<ApplyOutcome> {
    frame
        .mark_running(step)
        .and_then(|_| frame.mark_succeeded(step))
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step,
            detail: "mark ask timeout resolved failed".to_owned(),
        })?;
    Ok(ApplyOutcome::Executed)
}

fn ensure_step_running(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
    context: &str,
) -> RecoveryResult<()> {
    let current = frame
        .step_state(step)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step,
            detail: "step_state read failed".to_owned(),
        })?;
    if current == vb_core::StepState::Pending {
        frame
            .mark_running(step)
            .map_err(|_e| RecoveryError::ReplayDivergence {
                step,
                detail: format!("mark_running before {context} failed"),
            })?;
    }
    Ok(())
}
