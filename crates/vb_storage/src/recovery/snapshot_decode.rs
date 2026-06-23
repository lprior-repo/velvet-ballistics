#![forbid(unsafe_code)]
//! Snapshot decoding and dimension derivation for journal recovery.
//!
//! Provides:
//! - `decode_snapshot_slots`: decodes and merges postcard-encoded slot/taint bytes
//! - `derive_dimensions_from_snapshot_and_tail`: computes step_count, slot_count, first_step
//!
//! These functions support snapshot-plus-tail hydration by extracting
//! structural information from pre-recorded journal state.

use crate::recovery::{RecoveredSlotEntry, RecoveryError, RecoveryResult, RunSnapshot};
use crate::{EventSeq, JournalEvent};
use vb_core::{RunId, SlotIdx, SlotValue, Taint};

/// Decodes snapshot slot/taint bytes into recovered slot entries.
///
/// Expects postcard-encoded `Vec<(SlotIdx, SlotValue)>` in the `slots` field
/// and `Vec<(SlotIdx, Taint)>` in the `taint` field. Explicit taint from the
/// taint vector overrides the default `Taint::Clean` carried by the slots
/// vector (which is a value-only projection, with no taint field at all).
pub(super) fn decode_snapshot_slots(
    slots_bytes: &[u8],
    taint_bytes: &[u8],
    run: RunId,
) -> RecoveryResult<Vec<RecoveredSlotEntry>> {
    if slots_bytes.is_empty() && taint_bytes.is_empty() {
        return Ok(Vec::new());
    }

    let slots: Vec<(SlotIdx, SlotValue)> =
        postcard::from_bytes(slots_bytes).map_err(|_| RecoveryError::CorruptSnapshot {
            run,
            seq: EventSeq::new(0),
        })?;

    let taint: Vec<(SlotIdx, Taint)> =
        postcard::from_bytes(taint_bytes).map_err(|_| RecoveryError::CorruptSnapshot {
            run,
            seq: EventSeq::new(0),
        })?;

    // Merge slots and taint, preferring explicit taint from the taint vector.
    // SR-019: slots and taint are now distinct projections, so a divergent
    // taint entry actually carries a payload that diverges from the default
    // `Taint::Clean` we synthesize for slots-only entries.
    let mut entries = Vec::with_capacity(slots.len());
    for (slot, value) in slots {
        let explicit_taint = taint
            .iter()
            .find_map(|(t_slot, t_taint)| {
                if *t_slot == slot {
                    Some(*t_taint)
                } else {
                    None
                }
            })
            .unwrap_or(Taint::Clean);
        entries.push(RecoveredSlotEntry {
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
/// Computes max step/slot indices from both snapshot entries and tail events,
/// then converts to counts with overflow protection.
pub(super) fn derive_dimensions_from_snapshot_and_tail(
    _snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run: RunId,
    snapshot_slots: &[RecoveredSlotEntry],
) -> RecoveryResult<(u16, u16, vb_core::StepIdx)> {
    let mut max_step: Option<vb_core::StepIdx> = None;
    let mut min_step: Option<vb_core::StepIdx> = None;
    let mut max_slot: Option<SlotIdx> = None;

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
            | JournalEvent::ActionScheduled { step, .. }
            | JournalEvent::ActionCompletedEvent { step, .. }
            | JournalEvent::ActionFailedEvent { step, .. }
            | JournalEvent::WaitScheduledEvent { step, .. }
            | JournalEvent::AskScheduledEvent { step, .. }
            | JournalEvent::RetryScheduledEvent { step, .. } => {
                max_step = Some(max_step.map_or(*step, |s| s.max(*step)));
                min_step = Some(min_step.map_or(*step, |s| s.min(*step)));
            }
            // SR-005 / vb-xb38b: StepSucceeded writes its result into `output`,
            // so the slot dimension must cover that index even when no other
            // slot-bearing events preceded it in the tail. Mirrors the
            // ActionCompletedEnvelope treatment so the two cannot drift.
            JournalEvent::StepSucceeded { step, output, .. } => {
                max_step = Some(max_step.map_or(*step, |s| s.max(*step)));
                min_step = Some(min_step.map_or(*step, |s| s.min(*step)));
                max_slot = Some(max_slot.map_or(*output, |s| s.max(*output)));
            }
            JournalEvent::ActionScheduledTicket { ticket, output, .. } => {
                max_step = Some(max_step.map_or(ticket.step, |s| s.max(ticket.step)));
                min_step = Some(min_step.map_or(ticket.step, |s| s.min(ticket.step)));
                // SR-005: ticket output is the slot the action will write to, so the
                // frame must reserve capacity for it even before the envelope lands.
                max_slot = Some(max_slot.map_or(*output, |s| s.max(*output)));
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
            // SR-005: RunAnswered writes the answer into `slot_idx` and so requires
            // the slot dimension to cover that index even when no other slot
            // events preceded it in the tail.
            JournalEvent::RunAnswered { slot_idx, .. } => {
                max_slot = Some(max_slot.map_or(*slot_idx, |s| s.max(*slot_idx)));
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
