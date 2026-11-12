//! Borrowed, already-prepared input context for deterministic cold replay.

use std::cell::Cell;

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

#[path = "input_context/owned.rs"]
mod owned;
pub use owned::{
    RecoveredReplayOutcome, recover_runtime_frame_seed_from_events_with_replay_outcome,
};

#[derive(Clone, Copy)]
enum ReplayStoreMode<'a> {
    RetainedOnly,
    OwnReplay(&'a Cell<Option<ValueStore>>),
}

enum ReplayFrameOutcome {
    ThroughTarget(RunFrame, ValueStore),
    BeforeTargetFallback(RunFrame, ValueStore),
}

impl ReplayFrameOutcome {
    fn frame(&self) -> &RunFrame {
        match self {
            Self::ThroughTarget(frame, _) | Self::BeforeTargetFallback(frame, _) => frame,
        }
    }

    fn store_ref(&self) -> &ValueStore {
        match self {
            Self::ThroughTarget(_, store) | Self::BeforeTargetFallback(_, store) => store,
        }
    }

    fn into_store(self) -> ValueStore {
        match self {
            Self::ThroughTarget(_, store) | Self::BeforeTargetFallback(_, store) => store,
        }
    }

    fn successor_cursor(&self) -> Option<(StepIdx, u64)> {
        match self {
            Self::ThroughTarget(frame, _) => Some((frame.pc(), frame.executed())),
            Self::BeforeTargetFallback(_, _) => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ReplayInputContext<'a> {
    inputs: &'a [(SlotIdx, SlotValue, Taint)],
    store: Option<&'a ValueStore>,
    mode: ReplayStoreMode<'a>,
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
    let context = ReplayInputContext {
        inputs,
        store,
        mode: ReplayStoreMode::RetainedOnly,
    };
    recover_runtime_frame_seed_with_context(events, Some(workflow), Some(context))
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
        Some(context) => classify_slots_for_context(entries, context, &outcome),
        None => RecoveredSlots::from_replayed(entries),
    };
    let cursor = if context.is_some() && slots.fully_supported {
        outcome.successor_cursor()
    } else {
        None
    };
    deposit_outcome_store(outcome, context);
    Ok((slots, cursor))
}

fn classify_slots_for_context(
    entries: Vec<RecoveredSlotEntry>,
    context: ReplayInputContext<'_>,
    outcome: &ReplayFrameOutcome,
) -> RecoveredSlots {
    match context.mode {
        ReplayStoreMode::RetainedOnly => context.classify_retained_only(entries),
        ReplayStoreMode::OwnReplay(_) => validate_against_outcome_store(entries, outcome),
    }
}

fn validate_against_outcome_store(
    entries: Vec<RecoveredSlotEntry>,
    outcome: &ReplayFrameOutcome,
) -> RecoveredSlots {
    let store = outcome.store_ref();
    let all_valid = entries
        .iter()
        .all(|entry| validate_reference(store, entry.value).is_ok());
    if all_valid {
        RecoveredSlots::supported(entries)
    } else {
        let mut slots = RecoveredSlots::unsupported();
        slots.entries = entries;
        slots
    }
}

fn deposit_outcome_store(outcome: ReplayFrameOutcome, context: Option<ReplayInputContext<'_>>) {
    if let Some(ctx) = context {
        if let ReplayStoreMode::OwnReplay(sink) = ctx.mode {
            sink.set(Some(outcome.into_store()));
        }
    }
}

fn replay_frame(
    plan: &CompiledWorkflow,
    target: StepIdx,
    context: Option<ReplayInputContext<'_>>,
) -> RecoveryResult<ReplayFrameOutcome> {
    let engine = build_engine(plan, context);
    let mut store = scratch_store(plan, context)?;
    match engine.replay_frame_through(target, &mut store) {
        Ok(frame) => Ok(ReplayFrameOutcome::ThroughTarget(frame, store)),
        Err(ReplayError::NonDeterministicStep { step, .. }) if step == target => {
            let mut store = scratch_store(plan, context)?;
            engine
                .replay_frame_up_to(target, &mut store)
                .map(|frame| ReplayFrameOutcome::BeforeTargetFallback(frame, store))
                .map_err(replay_error_to_recovery)
        }
        Err(error) => Err(replay_error_to_recovery(error)),
    }
}

fn build_engine<'a>(
    plan: &'a CompiledWorkflow,
    context: Option<ReplayInputContext<'a>>,
) -> ReplayEngine<'a> {
    let engine = ReplayEngine::new(plan);
    match context {
        Some(context) => engine.with_initial_inputs(context.inputs),
        None => engine,
    }
}

fn scratch_store(
    plan: &CompiledWorkflow,
    context: Option<ReplayInputContext<'_>>,
) -> RecoveryResult<ValueStore> {
    let store = make_scratch_store(plan, context)?;
    if let Some(context) = context {
        validate_input_references(plan, context.inputs, &store)?;
    }
    Ok(store)
}

fn make_scratch_store(
    plan: &CompiledWorkflow,
    context: Option<ReplayInputContext<'_>>,
) -> RecoveryResult<ValueStore> {
    match context.and_then(|context| context.store) {
        Some(store) => ValueStore::from_checkpoint_v1(store.checkpoint_v1()).map_err(context_error),
        None => match context.map(|context| context.mode) {
            Some(ReplayStoreMode::OwnReplay(_)) => owned::fresh_store(plan),
            _ => Ok(ValueStore::new()),
        },
    }
}

fn validate_input_references(
    plan: &CompiledWorkflow,
    inputs: &[(SlotIdx, SlotValue, Taint)],
    store: &ValueStore,
) -> RecoveryResult<()> {
    for (slot, value, _taint) in inputs {
        if slot.get() >= plan.slot_count() {
            return Err(replay_error_to_recovery(ReplayError::SlotNotAvailable {
                slot: *slot,
            }));
        }
        validate_reference(store, *value).map_err(context_error)?;
    }
    Ok(())
}

impl ReplayInputContext<'_> {
    fn classify_retained_only(self, entries: Vec<RecoveredSlotEntry>) -> RecoveredSlots {
        match self.store {
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
