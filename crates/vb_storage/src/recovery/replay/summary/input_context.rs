//! Borrowed, already-prepared input context for deterministic cold replay.

use super::derive::{recover_runtime_frame_seed_with_context, reject_workflow_digest_mismatch};
use super::hydrate::{RecoveredSlots, initialized_recovered_slots, replay_error_to_recovery};
use crate::JournalEvent;
use crate::recovery::types::{
    RecoveredSlotEntry, RecoveryError, RecoveryFrameSeedProduct, RecoveryResult,
};
use vb_core::replay::{ReplayEngine, ReplayError};
use vb_core::{
    CompiledWorkflow, CoreError, RunFrame, SlotIdx, SlotValue, StepIdx, Taint, ValueStore,
};

enum ReplayFrameOutcome {
    ThroughTarget(RunFrame),
    BeforeTargetFallback(RunFrame),
}

impl ReplayFrameOutcome {
    fn frame(&self) -> &RunFrame {
        match self {
            Self::ThroughTarget(frame) | Self::BeforeTargetFallback(frame) => frame,
        }
    }

    fn successor_cursor(&self) -> Option<(StepIdx, u64)> {
        match self {
            Self::ThroughTarget(frame) => Some((frame.pc(), frame.executed())),
            Self::BeforeTargetFallback(_) => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ReplayInputContext<'a> {
    inputs: &'a [(SlotIdx, SlotValue, Taint)],
    store: Option<&'a ValueStore>,
}

/// Replays using retained roots and an already-prepared store, without remapping.
/// Callers retain original record, workflow, authority and taint validation.
pub fn recover_runtime_frame_seed_from_events_with_replay_inputs(
    events: &[JournalEvent],
    workflow: &CompiledWorkflow,
    inputs: &[(SlotIdx, SlotValue, Taint)],
    store: Option<&ValueStore>,
) -> RecoveryResult<(RecoveryFrameSeedProduct, Option<(StepIdx, u64)>)> {
    reject_workflow_digest_mismatch(events, workflow.digest())?;
    recover_runtime_frame_seed_with_context(
        events,
        Some(workflow),
        Some(ReplayInputContext { inputs, store }),
    )
    .map(|(seed, cursor)| (RecoveryFrameSeedProduct::from_seed(seed), cursor))
}

pub(super) fn recover_slots_through_step(
    plan: &CompiledWorkflow,
    target: StepIdx,
    context: Option<ReplayInputContext<'_>>,
) -> RecoveryResult<(RecoveredSlots, Option<(StepIdx, u64)>)> {
    let outcome = replay_frame(plan, target, context)?;
    let entries = initialized_recovered_slots(outcome.frame(), target)?;
    let slots = match context {
        Some(context) => context.classify_slots(entries),
        None => RecoveredSlots::from_replayed(entries),
    };
    let cursor = if context.is_some() && slots.fully_supported {
        outcome.successor_cursor()
    } else {
        None
    };
    Ok((slots, cursor))
}

fn replay_frame(
    plan: &CompiledWorkflow,
    target: StepIdx,
    context: Option<ReplayInputContext<'_>>,
) -> RecoveryResult<ReplayFrameOutcome> {
    let engine = ReplayEngine::new(plan);
    let engine = match context {
        Some(context) => engine.with_initial_inputs(context.inputs),
        None => engine,
    };
    let mut store = scratch_store(plan, context)?;
    match engine.replay_frame_through(target, &mut store) {
        Ok(frame) => Ok(ReplayFrameOutcome::ThroughTarget(frame)),
        Err(ReplayError::NonDeterministicStep { step, .. }) if step == target => {
            let mut store = scratch_store(plan, context)?;
            engine
                .replay_frame_up_to(target, &mut store)
                .map(ReplayFrameOutcome::BeforeTargetFallback)
                .map_err(replay_error_to_recovery)
        }
        Err(error) => Err(replay_error_to_recovery(error)),
    }
}

fn scratch_store(
    plan: &CompiledWorkflow,
    context: Option<ReplayInputContext<'_>>,
) -> RecoveryResult<ValueStore> {
    let store = match context.and_then(|context| context.store) {
        Some(store) => {
            ValueStore::from_checkpoint_v1(store.checkpoint_v1()).map_err(context_error)?
        }
        None => ValueStore::new(),
    };
    if let Some(context) = context {
        context.inputs.iter().try_for_each(|(slot, value, _)| {
            if slot.get() >= plan.slot_count() {
                return Err(replay_error_to_recovery(ReplayError::SlotNotAvailable {
                    slot: *slot,
                }));
            }
            validate_reference(&store, *value).map_err(context_error)
        })?;
    }
    Ok(store)
}

impl ReplayInputContext<'_> {
    fn classify_slots(self, entries: Vec<RecoveredSlotEntry>) -> RecoveredSlots {
        match self.store {
            // Resolve against the retained store, never a scratch arena extended
            // by replay: those new IDs are not retained runtime authority.
            Some(store)
                if entries
                    .iter()
                    .all(|entry| validate_reference(store, entry.value).is_ok()) =>
            {
                RecoveredSlots::supported(entries)
            }
            Some(_) => RecoveredSlots::unsupported(),
            None => RecoveredSlots::from_replayed(entries),
        }
    }
}

fn validate_reference(store: &ValueStore, value: SlotValue) -> Result<(), CoreError> {
    match value {
        SlotValue::Symbol(id) => store.symbol(id).map(|_| ()),
        SlotValue::List(id) => store.list(id).map(|_| ()),
        SlotValue::Object(id) => store.object(id).map(|_| ()),
        SlotValue::Blob(id) => store.blob(id).map(|_| ()),
        SlotValue::Null | SlotValue::Bool(_) | SlotValue::I64(_) | SlotValue::F64(_) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "unsupported replay input value".into(),
        }),
    }
}

fn context_error(error: CoreError) -> RecoveryError {
    RecoveryError::ReplayDivergence {
        step: StepIdx::ZERO,
        detail: format!("invalid replay input context: {error}"),
    }
}
