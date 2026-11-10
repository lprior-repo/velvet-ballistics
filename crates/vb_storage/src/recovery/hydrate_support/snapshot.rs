use std::collections::HashMap;

use crate::recovery::types::{RecoveryError, RecoveryResult, RunSnapshot};
use crate::{EventSeq, JournalEvent};
use vb_core::RunId;

/// Decodes snapshot slot/taint bytes into recovered slot entries.
///
/// Expects postcard-encoded `Vec<(SlotIdx, SlotValue, Taint)>` in the slots field,
/// and the same format in the taint field (used for validation/merge).
pub(crate) fn decode_snapshot_slots(
    slots_bytes: &[u8],
    taint_bytes: &[u8],
    run: RunId,
    seq: EventSeq,
) -> RecoveryResult<Vec<crate::recovery::types::RecoveredSlotEntry>> {
    if slots_bytes.is_empty() && taint_bytes.is_empty() {
        return Ok(Vec::new());
    }

    let slots: Vec<(vb_core::SlotIdx, vb_core::SlotValue, vb_core::Taint)> =
        postcard::from_bytes(slots_bytes)
            .map_err(|_| RecoveryError::CorruptSnapshot { run, seq })?;

    let taint: Vec<(vb_core::SlotIdx, vb_core::SlotValue, vb_core::Taint)> =
        postcard::from_bytes(taint_bytes)
            .map_err(|_| RecoveryError::CorruptSnapshot { run, seq })?;

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
pub(crate) fn derive_dimensions_from_snapshot_and_tail(
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
            | JournalEvent::StepFailed { step, .. }
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
