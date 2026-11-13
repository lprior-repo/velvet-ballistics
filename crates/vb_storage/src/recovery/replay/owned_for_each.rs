#![forbid(unsafe_code)]
//! Deterministic owned-arena trace replay.
//!
//! The generic recovery path replays a workflow from an empty frame/store and
//! therefore cannot reconstruct post-admission `List`/`Object` handles
//! (ForEach cursor and accumulator, Reduce accumulator, Together branch
//! results, Repeat materialization). This module consumes the exact journal
//! activation order starting from the owned input checkpoint, so the replay
//! value store matches the live one handle-for-handle.

use vb_core::frame::StepState;
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{
    CompiledNodeKind as K, CompiledWorkflow, RunFrame, SlotIdx, SlotValue, StepIdx, Taint,
};

use super::owned_for_each_summary::{pending_actions, summary};
use crate::JournalEvent;
use crate::recovery::complete_scalar::driver;
use crate::recovery::complete_scalar::{TraceReplayInput, TraceReplayMode};
use crate::recovery::types::{
    RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState, RecoveryError, RecoveryFrameSeed,
    RecoveryFrameSeedProduct, RecoveryResult, UnsupportedRecoveryState,
};

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

/// Returns true when `plan` is inside the deterministic owned-arena
/// contract handled by [`replay_owned_for_each`].
#[must_use]
pub fn is_owned_for_each_contract(plan: &CompiledWorkflow) -> bool {
    eligible(plan)
}

/// Replays a deterministic arena trace from the journal activation order.
/// Supports the ForEach, Reduce, Together, and Repeat families.
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
    if !is_owned_for_each_contract(plan)
        || events
            .iter()
            .any(super::owned_for_each_summary::unsupported_replay_event)
    {
        return Ok(None);
    }
    if store_checkpoint.is_none() && retained_handles_require_store(initial_inputs) {
        return Ok(None);
    }
    let (frame, store) = driver::replay_with_inputs_and_store(
        events,
        plan,
        &TraceReplayInput {
            initial_inputs,
            store_checkpoint,
            mode: TraceReplayMode::OwnedArena,
        },
    )?;
    let product = seed_from_frame(events, plan, &frame)?;
    Ok(Some(OwnedForEachReplay {
        product,
        frame,
        store: store.checkpoint_v1(),
    }))
}

fn retained_handles_require_store(initial_inputs: &[(SlotIdx, SlotValue, Taint)]) -> bool {
    initial_inputs.iter().any(|(_, value, _)| {
        matches!(
            value,
            SlotValue::Symbol(_) | SlotValue::List(_) | SlotValue::Object(_) | SlotValue::Blob(_)
        )
    })
}

fn eligible(plan: &CompiledWorkflow) -> bool {
    let mut saw_owned_control = false;
    for raw in 0..plan.node_count() {
        let step = StepIdx::new(raw);
        let Some(node) = plan.node(step) else {
            return false;
        };
        if node.routing.error.handler().is_some() || node.routing.error.error_slot().is_some() {
            return false;
        }
        match node.kind {
            K::ForEachStart { .. }
            | K::ForEachNext { .. }
            | K::ForEachJoin { .. }
            | K::ReduceStart { .. }
            | K::ReduceNext { .. }
            | K::ReduceFinish { .. }
            | K::TogetherStart { .. }
            | K::TogetherBranch { .. }
            | K::TogetherJoin { .. }
            | K::RepeatStart { .. }
            | K::RepeatAttempt { .. }
            | K::RepeatCheck { .. }
            | K::RepeatFinish { .. } => {
                saw_owned_control = true;
            }
            K::Ask { .. } | K::AskResume { .. } => {}
            K::Do { .. } | K::WaitUntil { .. } | K::WaitEvent { .. } => {}
            K::Nop
            | K::SetConst { .. }
            | K::Copy { .. }
            | K::EvalExpr { .. }
            | K::BuildObject { .. }
            | K::BuildList { .. }
            | K::Choose { .. }
            | K::ChooseSlot { .. }
            | K::Jump { .. }
            | K::Finish { .. } => {}
            _ => return false,
        }
    }
    saw_owned_control
}

fn seed_from_frame(
    events: &[JournalEvent],
    plan: &CompiledWorkflow,
    frame: &RunFrame,
) -> RecoveryResult<RecoveryFrameSeedProduct> {
    let summary = summary(events);
    let mut steps = Vec::new();
    for raw in 0..plan.node_count() {
        let step = StepIdx::new(raw);
        let state = frame
            .step_state(step)
            .map_err(|error| RecoveryError::ReplayDivergence {
                step,
                detail: format!("for_each replay state: {error}"),
            })?;
        let recovered = match state {
            StepState::Pending => continue,
            StepState::Running => RecoveredStepState::Running,
            StepState::Succeeded => RecoveredStepState::Succeeded,
            StepState::Failed => RecoveredStepState::Failed,
            StepState::Waiting => RecoveredStepState::Waiting,
            StepState::Asking => RecoveredStepState::Asking,
            StepState::Skipped | StepState::Cancelled => {
                return Err(RecoveryError::ReplayDivergence {
                    step,
                    detail: String::from("unsupported for_each replay step state"),
                });
            }
            _ => {
                return Err(RecoveryError::ReplayDivergence {
                    step,
                    detail: String::from("unknown for_each replay step state"),
                });
            }
        };
        steps.push(RecoveredStepEntry {
            step,
            state: recovered,
        });
    }
    let slots = frame
        .initialized_slots()
        .map_err(|error| RecoveryError::ReplayDivergence {
            step: frame.pc(),
            detail: format!("for_each replay slots: {error}"),
        })?
        .into_iter()
        .map(|(slot, value, taint)| RecoveredSlotEntry { slot, value, taint })
        .collect();
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
