#![forbid(unsafe_code)]
//! Slot recovery, taint extraction, pending actions, and replay error mapping.
//!
//! Provides:
//! - `RecoveredSlots` — typed container for recovered slot entries + support flag
//! - `RecoveredSlotTaint` — taint with unsafety flag
//! - `recovered_slot_taint` — slot taint from event data
//! - `recover_slots` — event → slot entries, optionally merged with workflow replay
//! - `pending_actions_from_events` — pending action set from journal events
//! - `replay_error_to_recovery` — ReplayError → RecoveryError mapping

use std::collections::HashSet;

use crate::recovery::types::ActionReplayEffect;
use crate::recovery::{
    ActionReplayTracker, RecoveredPendingAction, RecoveredSlotEntry, RecoveryError, RecoveryResult,
};
use crate::slot_extra::{DecodedSlotWrittenExtra, decode_slot_written_extra};
use crate::{EventSeq, JournalEvent};
use vb_core::replay::ReplayError;
use vb_core::value_store::ValueStore;
use vb_core::{ActionId, CompiledWorkflow, RunId, SlotIdx, SlotValue, StepIdx, Taint};

// ── RecoveredSlots ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecoveredSlots {
    pub(super) entries: Vec<RecoveredSlotEntry>,
    pub(super) fully_supported: bool,
}

impl RecoveredSlots {
    pub(super) fn supported(entries: Vec<RecoveredSlotEntry>) -> Self {
        Self {
            entries,
            fully_supported: true,
        }
    }

    pub(super) fn unsupported() -> Self {
        Self {
            entries: Vec::new(),
            fully_supported: false,
        }
    }

    pub(super) fn from_replayed(entries: Vec<RecoveredSlotEntry>) -> Self {
        if entries
            .iter()
            .all(|entry| recoverable_slot_value(entry.value))
        {
            Self::supported(entries)
        } else {
            Self::unsupported()
        }
    }
}

fn recoverable_slot_value(value: SlotValue) -> bool {
    matches!(
        value,
        SlotValue::Null
            | SlotValue::Bool(_)
            | SlotValue::I64(_)
            | SlotValue::F64(_)
            | SlotValue::Symbol(_)
    )
}

// ── Slot recovery ───────────────────────────────────────────────────────────

pub(super) fn recover_slots(
    accumulator: &super::frame_seed::FrameSeedAccumulator,
    workflow: Option<&CompiledWorkflow>,
) -> RecoveryResult<RecoveredSlots> {
    match (workflow, accumulator.last_succeeded_step) {
        (Some(plan), Some(target)) => Ok(merge_recovered_slots(
            recover_slots_through_step(plan, target)?,
            recovered_event_slots(accumulator),
        )),
        (Some(_), None) => Ok(RecoveredSlots::supported(Vec::new())),
        (None, _) if accumulator.slot_values.is_empty() => Ok(RecoveredSlots::unsupported()),
        (None, _) => Ok(RecoveredSlots::supported(recovered_event_slots(
            accumulator,
        ))),
    }
}

fn merge_recovered_slots(
    mut base: RecoveredSlots,
    overrides: Vec<RecoveredSlotEntry>,
) -> RecoveredSlots {
    for override_entry in overrides {
        match base
            .entries
            .iter_mut()
            .find(|entry| entry.slot == override_entry.slot)
        {
            Some(entry) => *entry = override_entry,
            None => base.entries.push(override_entry),
        }
    }
    base
}

fn recovered_event_slots(
    accumulator: &super::frame_seed::FrameSeedAccumulator,
) -> Vec<RecoveredSlotEntry> {
    accumulator
        .slot_values
        .iter()
        .map(|(slot, value)| RecoveredSlotEntry {
            slot: *slot,
            value: *value,
            taint: accumulator
                .slot_taint
                .get(slot)
                .copied()
                .map_or(Taint::Secret, |taint| taint),
        })
        .collect()
}

fn recover_slots_through_step(
    plan: &CompiledWorkflow,
    target: StepIdx,
) -> RecoveryResult<RecoveredSlots> {
    let mut store = ValueStore::new();
    let engine = vb_core::replay::ReplayEngine::new(plan);
    let frame = match engine.replay_frame_through(target, &mut store) {
        Ok(frame) => frame,
        Err(ReplayError::NonDeterministicStep { step, .. }) if step == target => {
            let mut store = ValueStore::new();
            engine
                .replay_frame_up_to(target, &mut store)
                .map_err(replay_error_to_recovery)?
        }
        Err(error) => return Err(replay_error_to_recovery(error)),
    };
    let slots = initialized_recovered_slots(&frame, target)?;
    Ok(RecoveredSlots::from_replayed(slots))
}

fn initialized_recovered_slots(
    frame: &vb_core::RunFrame,
    target: StepIdx,
) -> RecoveryResult<Vec<RecoveredSlotEntry>> {
    Ok(frame
        .initialized_slots()
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: target,
            detail: "replay produced invalid slot evidence".to_owned(),
        })?
        .into_iter()
        .map(|(slot, value, taint)| RecoveredSlotEntry { slot, value, taint })
        .collect::<Vec<_>>())
}

// ── Slot taint recovery ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RecoveredSlotTaint {
    pub(super) taint: Taint,
    pub(super) unsupported: bool,
}

pub(super) fn recovered_slot_taint(
    slot: SlotIdx,
    value: SlotValue,
    extra: &Option<Vec<u8>>,
) -> RecoveryResult<RecoveredSlotTaint> {
    match extra {
        Some(bytes) => decoded_slot_taint(slot, value, bytes),
        None => Ok(legacy_recovered_slot_taint(value)),
    }
}

fn decoded_slot_taint(
    slot: SlotIdx,
    value: SlotValue,
    bytes: &[u8],
) -> RecoveryResult<RecoveredSlotTaint> {
    match decode_slot_written_extra(bytes) {
        Ok(DecodedSlotWrittenExtra::Envelope(envelope)) => Ok(RecoveredSlotTaint {
            taint: envelope.taint,
            unsupported: false,
        }),
        Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_)) => {
            Ok(legacy_frame_extra_recovered_slot_taint(value))
        }
        Err(_) => Err(RecoveryError::CorruptSlotTaint { slot }),
    }
}

fn legacy_recovered_slot_taint(value: SlotValue) -> RecoveredSlotTaint {
    RecoveredSlotTaint {
        taint: legacy_slot_taint(value),
        unsupported: false,
    }
}

fn legacy_frame_extra_recovered_slot_taint(value: SlotValue) -> RecoveredSlotTaint {
    RecoveredSlotTaint {
        taint: legacy_slot_taint(value),
        unsupported: true,
    }
}

fn legacy_slot_taint(value: SlotValue) -> Taint {
    match value {
        SlotValue::Bool(false) => Taint::Clean,
        SlotValue::Bool(true) | SlotValue::Null => Taint::DerivedFromSecret,
        _ => Taint::Secret,
    }
}

// ── Pending actions ─────────────────────────────────────────────────────────

/// Production proof surface: converts the accumulator HashSet into the
/// public-facing `Vec<RecoveredPendingAction>` representation.
fn recovered_pending_actions(
    pending_actions: HashSet<(ActionId, StepIdx)>,
) -> Vec<RecoveredPendingAction> {
    pending_actions
        .into_iter()
        .map(|(action, step)| RecoveredPendingAction { step, action })
        .collect()
}

/// Public accessor for tests and observability.
/// Returns the set of actions that were scheduled but not completed
/// from a sequence of journal events.
///
/// This is a convenience wrapper around the private `recovered_pending_actions`
/// that accepts raw journal events instead of a pre-built accumulator.
#[must_use]
pub fn pending_actions_from_events(events: &[JournalEvent]) -> Vec<RecoveredPendingAction> {
    let accumulator = recover_pending_actions_from_events_inner(events);
    recovered_pending_actions(accumulator)
}

fn recover_pending_actions_from_events_inner(
    events: &[JournalEvent],
) -> HashSet<(ActionId, StepIdx)> {
    let mut pending: HashSet<(ActionId, StepIdx)> = HashSet::new();

    for event in events {
        match event {
            JournalEvent::ActionScheduled { step, action, .. } => {
                pending.insert((*action, *step));
            }
            JournalEvent::ActionScheduledTicket { ticket, .. } => {
                pending.insert((ticket.action, ticket.step));
            }
            JournalEvent::ActionCompletedEvent { step, action, .. } => {
                pending.remove(&(*action, *step));
            }
            JournalEvent::ActionCompletedEnvelope { ticket, .. } => {
                pending.remove(&(ticket.action, ticket.step));
            }
            // All other events are irrelevant for pending actions tracking
            _ => {}
        }
    }

    pending
}

// ── Replay error mapping ───────────────────────────────────────────────────

pub(crate) fn replay_error_to_recovery(error: ReplayError) -> RecoveryError {
    match error {
        ReplayError::StepNotFound { step } => RecoveryError::ReplayDivergence {
            step,
            detail: "replay step not found in compiled workflow".to_owned(),
        },
        ReplayError::NonDeterministicStep { step, kind } => RecoveryError::ReplayDivergence {
            step,
            detail: format!("replay blocked by non-deterministic {kind} step"),
        },
        ReplayError::SlotNotAvailable { slot } => RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: format!("replay required unavailable slot {:?}", slot),
        },
        ReplayError::ExpressionEvalFailed { step } => RecoveryError::ReplayDivergence {
            step,
            detail: "replay expression evaluation failed".to_owned(),
        },
        ReplayError::Internal { reason } => RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: reason.to_owned(),
        },
        // `ReplayError` is `#[non_exhaustive]`; unknown variants
        // map to a generic replay divergence error.
        _ => RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: "unknown replay error".to_owned(),
        },
    }
}
