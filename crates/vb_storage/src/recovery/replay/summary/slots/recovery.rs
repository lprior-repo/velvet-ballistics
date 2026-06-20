#![forbid(unsafe_code)]
//! RecoveredSlots type and slot recovery from events/workflow.
//!
//! Provides:
//! - `RecoveredSlots` — typed container for recovered slot entries + support flag
//! - `recover_slots` — event → slot entries, optionally merged with workflow replay

use crate::recovery::replay::summary::frame_seed::FrameSeedAccumulator;
use crate::recovery::{RecoveredSlotEntry, RecoveryError, RecoveryResult};
use crate::slot_extra::{DecodedSlotWrittenExtra, decode_slot_written_extra};
use vb_core::replay::ReplayError;
use vb_core::value_store::ValueStore;
use vb_core::{CompiledWorkflow, RunId, SlotIdx, SlotValue, StepIdx, Taint};

// ── RecoveredSlots ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecoveredSlots {
    pub(crate) entries: Vec<RecoveredSlotEntry>,
    pub(crate) fully_supported: bool,
}

impl RecoveredSlots {
    pub(super) fn supported(entries: Vec<RecoveredSlotEntry>) -> Self {
        Self {
            entries,
            fully_supported: true,
        }
    }

    pub(crate) fn unsupported() -> Self {
        Self {
            entries: Vec::new(),
            fully_supported: false,
        }
    }

    pub(crate) fn from_replayed(entries: Vec<RecoveredSlotEntry>) -> Self {
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

pub(crate) fn recover_slots(
    accumulator: &FrameSeedAccumulator,
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

fn recovered_event_slots(accumulator: &FrameSeedAccumulator) -> Vec<RecoveredSlotEntry> {
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
                .map_err(super::errors::replay_error_to_recovery)?
        }
        Err(error) => return Err(super::errors::replay_error_to_recovery(error)),
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
