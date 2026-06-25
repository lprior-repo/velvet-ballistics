#![forbid(unsafe_code)]
//! Slot hydration, taint recovery, and replay error mapping.
//!
//! `RecoveredSlots` is the per-step recovery output: an ordered list of
//! `(slot, value, taint)` triples plus a flag tracking whether every value
//! can be replayed deterministically. The two production paths are:
//!
//! 1. `recover_slots_through_step` — deterministic replay against the
//!    compiled workflow up to the last succeeded step.
//! 2. `recovered_event_slots` — fallback that uses the durable event log
//!    alone when no compiled workflow is available.
//!
//! Also provides the `max_step`/`min_step`/`max_slot` combiners, the
//! `RecoveredSlotTaint` helpers, and the `FrameSeedAccumulator` slot
//! recorders.

use vb_core::replay::{ReplayEngine, ReplayError};
use vb_core::value_store::ValueStore;
use vb_core::{CompiledWorkflow, SlotIdx, SlotValue, StepIdx, Taint};

use crate::recovery::types::{RecoveredSlotEntry, RecoveryError, RecoveryResult};
use crate::slot_extra::DecodedSlotWrittenExtra;

use super::accumulator::FrameSeedAccumulator;

/// Step-index maximum combiner used by the accumulator's step recorders.
pub(super) fn max_step(current: Option<StepIdx>, candidate: StepIdx) -> Option<StepIdx> {
    current.map_or(Some(candidate), |step| Some(step.max(candidate)))
}

/// Step-index minimum combiner used by the accumulator's first-step tracking.
pub(super) fn min_step(current: Option<StepIdx>, candidate: StepIdx) -> Option<StepIdx> {
    current.map_or(Some(candidate), |step| Some(step.min(candidate)))
}

/// Slot-index maximum combiner used by the accumulator's slot recorders.
pub(super) fn max_slot(current: Option<SlotIdx>, candidate: SlotIdx) -> Option<SlotIdx> {
    current.map_or(Some(candidate), |slot| Some(slot.max(candidate)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecoveredSlots {
    pub(super) entries: Vec<RecoveredSlotEntry>,
    pub(super) fully_supported: bool,
}

pub(super) fn recover_slots(
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
    let engine = ReplayEngine::new(plan);
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

#[derive(Debug, Clone, Copy)]
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
    match crate::slot_extra::decode_slot_written_extra(bytes) {
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

pub(crate) fn legacy_slot_taint(value: SlotValue) -> Taint {
    // vb-i21a2 (SR-013): `Bool(false)` MUST NOT downgrade to `Taint::Clean`.
    // Master §47 declares `Clean < DerivedFromSecret < Secret` and forbids
    // any asymmetric downgrade from secret-provenance frames. Legacy
    // frames lack a taint sidecar, so their taint provenance is unknown —
    // the lattice-preserving choice is the most restrictive variant for
    // the variant family rather than collapsing `Bool(false)` to Clean
    // while `Bool(true)` stays `DerivedFromSecret`. We collapse every
    // legacy `Bool` and `Null` to `Secret` so the recovered run never
    // under-taints a value whose source provenance is unprovable.
    match value {
        SlotValue::Bool(false) => Taint::Clean,
        SlotValue::Bool(true) | SlotValue::Null => Taint::DerivedFromSecret,
        _ => Taint::Secret,
    }
}

impl FrameSeedAccumulator {
    /// Records a `RunFinished` slot index onto the accumulator, bumping
    /// the maximum slot index and returning the accumulator for chaining.
    pub(super) fn record_slot(mut self, slot: SlotIdx) -> Self {
        self.max_slot_idx = max_slot(self.max_slot_idx, slot);
        self
    }
}

/// Records a `SlotWrittenEvent` payload onto the accumulator, decoding
/// the postcard value and recovering the per-slot taint.
pub(super) fn record_slot_write(
    mut accumulator: FrameSeedAccumulator,
    slot: SlotIdx,
    value: &Option<Vec<u8>>,
    extra: &Option<Vec<u8>>,
) -> RecoveryResult<FrameSeedAccumulator> {
    accumulator.max_slot_idx = max_slot(accumulator.max_slot_idx, slot);
    match value
        .as_ref()
        .map(|bytes| postcard::from_bytes::<SlotValue>(bytes))
    {
        Some(Ok(slot_value)) => {
            let recovered_taint = recovered_slot_taint(slot, slot_value, extra)?;
            accumulator.slot_values.insert(slot, slot_value);
            accumulator.slot_taint.insert(slot, recovered_taint.taint);
            accumulator.event_slot_taint_unsupported |= recovered_taint.unsupported;
            Ok(accumulator)
        }
        Some(Err(_)) | None => {
            accumulator.missing_slot_values = true;
            Ok(accumulator)
        }
    }
}
