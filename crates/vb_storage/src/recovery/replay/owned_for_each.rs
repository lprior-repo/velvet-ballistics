#![forbid(unsafe_code)]
//! Deterministic owned-arena trace replay from retained input checkpoints.
//!
//! Replays recorded activation order for supported bounded controls, including incremental ConcurrentV3 ForEach, preserving live arena handle allocation. Handles created after admission are reconstructed from the same retained input checkpoint and deterministic transitions.

use vb_core::frame::StepState;
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{CompiledWorkflow, RunFrame, SlotIdx, SlotValue, StepIdx, Taint};

use super::owned_for_each_summary::{pending_actions, summary};
use crate::JournalEvent;
use crate::recovery::complete_scalar::driver;
use crate::recovery::complete_scalar::driver::OwnedCollectReplaySnapshot;
use crate::recovery::complete_scalar::{TraceReplayInput, TraceReplayMode};
use crate::recovery::types::{
    RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState, RecoveryError, RecoveryFrameSeed,
    RecoveryFrameSeedProduct, RecoveryResult, UnsupportedRecoveryState,
};

#[path = "owned_for_each/supported_route.rs"]
mod supported_route;

/// A complete deterministic owned-arena replay.
#[derive(Debug)]
pub struct OwnedForEachReplay {
    /// Typed recovery product assembled from the replayed frame.
    pub product: RecoveryFrameSeedProduct,
    /// Replayed live frame at the last recorded step.
    pub frame: RunFrame,
    /// Value-store checkpoint that every recovered handle resolves against.
    pub store: ValueStoreCheckpointV1,
}

/// Owned-arena replay with transient Collect-state receipts for Runtime binding.
#[derive(Debug)]
pub struct OwnedForEachReplayWithCollectSnapshots {
    /// Existing deterministic replay product.
    pub replay: OwnedForEachReplay,
    /// Per-event Collect snapshots that must be checked against durable extras.
    pub collect_snapshots: Vec<OwnedCollectReplaySnapshot>,
}

/// Returns true when `plan` is inside the deterministic owned-arena
/// contract handled by [`replay_owned_for_each`].
#[must_use]
pub fn is_owned_for_each_contract(plan: &CompiledWorkflow) -> bool {
    supported_route::eligibility(plan) == Some(true)
}

/// Returns true for deterministic plans that need the retained owned input
/// arena even though they contain no compound-owned control node.
#[must_use]
pub fn is_owned_input_replay_contract(
    plan: &CompiledWorkflow,
    initial_inputs: &[(SlotIdx, SlotValue, Taint)],
    store_checkpoint: Option<&ValueStoreCheckpointV1>,
) -> bool {
    supported_route::eligibility(plan) == Some(false)
        && retained_handles_require_store(initial_inputs)
        && store_checkpoint.is_some()
}

/// Replays a deterministic arena trace from the journal activation order.
/// Supports the ForEach, Reduce, Together, Repeat, and Collect families.
///
/// Returns `None` when the workflow is outside this helper's deterministic
/// contract. A missing retained checkpoint is allowed when the driver can
/// initialize source metadata deterministically; retained arena handles
/// without their checkpoint stay on the existing fail-closed path.
pub fn replay_owned_for_each(
    events: &[JournalEvent],
    plan: &CompiledWorkflow,
    initial_inputs: &[(SlotIdx, SlotValue, Taint)],
    store_checkpoint: Option<&ValueStoreCheckpointV1>,
) -> RecoveryResult<Option<OwnedForEachReplay>> {
    replay_owned_for_each_with_collect_snapshots(events, plan, initial_inputs, store_checkpoint)
        .map(|replay| replay.map(|evidence| evidence.replay))
}

/// Replays an owned-arena trace and returns transient Collect-state evidence.
/// Callers that resume Collect must bind every receipt to its durable frame extra.
pub fn replay_owned_for_each_with_collect_snapshots(
    events: &[JournalEvent],
    plan: &CompiledWorkflow,
    initial_inputs: &[(SlotIdx, SlotValue, Taint)],
    store_checkpoint: Option<&ValueStoreCheckpointV1>,
) -> RecoveryResult<Option<OwnedForEachReplayWithCollectSnapshots>> {
    if !can_replay_owned_trace(events, plan, initial_inputs, store_checkpoint) {
        return Ok(None);
    }
    let (frame, store, collect_snapshots) =
        replay_owned_trace(events, plan, initial_inputs, store_checkpoint)?;
    let product = seed_from_frame(events, plan, &frame)?;
    Ok(Some(replay_evidence(
        product,
        frame,
        store,
        collect_snapshots,
    )))
}

fn can_replay_owned_trace(
    events: &[JournalEvent],
    plan: &CompiledWorkflow,
    initial_inputs: &[(SlotIdx, SlotValue, Taint)],
    store_checkpoint: Option<&ValueStoreCheckpointV1>,
) -> bool {
    let controls = is_owned_for_each_contract(plan);
    let source = is_owned_input_replay_contract(plan, initial_inputs, store_checkpoint);
    (controls || source)
        && !events
            .iter()
            .any(super::owned_for_each_summary::unsupported_replay_event)
        && !(store_checkpoint.is_none() && retained_handles_require_store(initial_inputs))
}

fn replay_owned_trace(
    events: &[JournalEvent],
    plan: &CompiledWorkflow,
    initial_inputs: &[(SlotIdx, SlotValue, Taint)],
    store_checkpoint: Option<&ValueStoreCheckpointV1>,
) -> RecoveryResult<(
    RunFrame,
    vb_core::ValueStore,
    Vec<OwnedCollectReplaySnapshot>,
)> {
    driver::replay_with_inputs_and_store_and_collect_snapshots(
        events,
        plan,
        &TraceReplayInput {
            initial_inputs,
            store_checkpoint,
            mode: TraceReplayMode::OwnedArena,
        },
    )
}

fn replay_evidence(
    product: RecoveryFrameSeedProduct,
    frame: RunFrame,
    store: vb_core::ValueStore,
    collect_snapshots: Vec<OwnedCollectReplaySnapshot>,
) -> OwnedForEachReplayWithCollectSnapshots {
    OwnedForEachReplayWithCollectSnapshots {
        replay: OwnedForEachReplay {
            product,
            frame,
            store: store.checkpoint_v1(),
        },
        collect_snapshots,
    }
}

fn retained_handles_require_store(initial_inputs: &[(SlotIdx, SlotValue, Taint)]) -> bool {
    initial_inputs.iter().any(|(_, value, _)| {
        matches!(
            value,
            SlotValue::Symbol(_) | SlotValue::List(_) | SlotValue::Object(_) | SlotValue::Blob(_)
        )
    })
}

fn seed_from_frame(
    events: &[JournalEvent],
    plan: &CompiledWorkflow,
    frame: &RunFrame,
) -> RecoveryResult<RecoveryFrameSeedProduct> {
    let summary = summary(events);
    let steps = recovered_steps(plan, frame)?;
    let slots = recovered_slots(frame)?;
    Ok(RecoveryFrameSeedProduct::from_seed(RecoveryFrameSeed {
        summary,
        first_step: plan.entry(),
        step_count: plan.node_count(),
        slot_count: plan.slot_count(),
        pc: frame.pc(),
        steps,
        slots,
        pending_actions: pending_actions(events),
        unsupported: UnsupportedRecoveryState::SUPPORTED,
    }))
}

fn recovered_steps(
    plan: &CompiledWorkflow,
    frame: &RunFrame,
) -> RecoveryResult<Vec<RecoveredStepEntry>> {
    let recovered: RecoveryResult<Vec<Option<RecoveredStepEntry>>> = (0..plan.node_count())
        .map(|raw| recovered_step(StepIdx::new(raw), frame))
        .collect();
    Ok(recovered?.into_iter().flatten().collect())
}

fn recovered_step(step: StepIdx, frame: &RunFrame) -> RecoveryResult<Option<RecoveredStepEntry>> {
    let state = frame
        .step_state(step)
        .map_err(|error| RecoveryError::ReplayDivergence {
            step,
            detail: format!("for_each replay state: {error}"),
        })?;
    let recovered = match state {
        StepState::Pending => return Ok(None),
        StepState::Running => RecoveredStepState::Running,
        StepState::Succeeded => RecoveredStepState::Succeeded,
        StepState::Failed => RecoveredStepState::Failed,
        StepState::Waiting => RecoveredStepState::Waiting,
        StepState::Asking => RecoveredStepState::Asking,
        StepState::Skipped | StepState::Cancelled => {
            return Err(unsupported_step_state(step));
        }
        _ => return Err(unknown_step_state(step)),
    };
    Ok(Some(RecoveredStepEntry {
        step,
        state: recovered,
    }))
}

fn unsupported_step_state(step: StepIdx) -> RecoveryError {
    RecoveryError::ReplayDivergence {
        step,
        detail: String::from("unsupported for_each replay step state"),
    }
}

fn unknown_step_state(step: StepIdx) -> RecoveryError {
    RecoveryError::ReplayDivergence {
        step,
        detail: String::from("unknown for_each replay step state"),
    }
}

fn recovered_slots(frame: &RunFrame) -> RecoveryResult<Vec<RecoveredSlotEntry>> {
    frame
        .initialized_slots()
        .map_err(|error| RecoveryError::ReplayDivergence {
            step: frame.pc(),
            detail: format!("for_each replay slots: {error}"),
        })
        .map(|slots| {
            slots
                .into_iter()
                .map(|(slot, value, taint)| RecoveredSlotEntry { slot, value, taint })
                .collect()
        })
}
