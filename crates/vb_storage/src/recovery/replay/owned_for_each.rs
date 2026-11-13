#![forbid(unsafe_code)]
//! Deterministic owned-arena trace replay.
//!
//! The generic recovery path replays a workflow from an empty frame/store and
//! therefore cannot reconstruct post-admission `List`/`Object` handles
//! (ForEach cursor and accumulator, Reduce accumulator, Together branch
//! results, Repeat materialization). This module consumes the exact journal
//! activation order starting from the owned input checkpoint, so the replay
//! value store matches the live one handle-for-handle.

use std::collections::BTreeMap;

use vb_core::frame::StepState;
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{
    ActionId, CompiledNodeKind as K, CompiledWorkflow, RunFrame, SlotIdx, SlotValue, StepIdx, Taint,
};

use crate::recovery::complete_scalar::driver;
use crate::recovery::complete_scalar::{TraceReplayInput, TraceReplayMode};
use crate::recovery::types::{
    RecoveredPendingAction, RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState,
    RecoveryError, RecoveryFrameSeed, RecoveryFrameSeedProduct, RecoveryResult,
    RecoveryRuntimeSummary, RecoveryTerminalState, UnsupportedRecoveryState,
};
use crate::{EventSeq, JournalEvent};

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

/// Replays an owned deterministic arena trace from the journal activation
/// order. Supports the ForEach, Reduce, Together, and Repeat families.
///
/// Returns `None` when the workflow is outside this helper's deterministic
/// contract, or when no owned input checkpoint is available, leaving the
/// existing recovery path unchanged.
pub fn replay_owned_for_each(
    events: &[JournalEvent],
    plan: &CompiledWorkflow,
    initial_inputs: &[(SlotIdx, SlotValue, Taint)],
    store_checkpoint: Option<&ValueStoreCheckpointV1>,
) -> RecoveryResult<Option<OwnedForEachReplay>> {
    if !is_owned_for_each_contract(plan) || !has_step_start(events) {
        return Ok(None);
    }
    let Some(store_checkpoint) = store_checkpoint else {
        return Ok(None);
    };
    let (frame, store) = driver::replay_with_inputs_and_store(
        events,
        plan,
        &TraceReplayInput {
            initial_inputs,
            store_checkpoint: Some(store_checkpoint),
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

fn has_step_start(events: &[JournalEvent]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, JournalEvent::StepStarted { .. }))
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

fn summary(events: &[JournalEvent]) -> RecoveryRuntimeSummary {
    let run = events
        .first()
        .map_or_else(|| vb_core::RunId::new(0), JournalEvent::run_id);
    let first_seq = events.first().map_or(EventSeq::ZERO, JournalEvent::seq);
    let last_seq = events.last().map_or(EventSeq::ZERO, JournalEvent::seq);
    let workflow = events.iter().find_map(|event| match event {
        JournalEvent::RunAccepted { workflow, .. } => Some(*workflow),
        _ => None,
    });
    let mut steps_started = 0_u64;
    let mut steps_succeeded = 0_u64;
    let mut actions_scheduled = 0_u64;
    let mut actions_resolved = 0_u64;
    let mut suspensions = 0_u64;
    let mut slots_written = 0_u64;
    let mut terminal = None;
    for event in events {
        match event {
            JournalEvent::StepStarted { .. } => steps_started = steps_started.saturating_add(1),
            JournalEvent::StepSucceeded { .. } => {
                steps_succeeded = steps_succeeded.saturating_add(1);
            }
            JournalEvent::ActionScheduled { .. } | JournalEvent::ActionScheduledTicket { .. } => {
                actions_scheduled = actions_scheduled.saturating_add(1);
            }
            JournalEvent::ActionCompletedEvent { .. } | JournalEvent::ActionFailedEvent { .. } => {
                actions_resolved = actions_resolved.saturating_add(1);
            }
            JournalEvent::ActionCompletedEnvelope { .. } => {
                actions_resolved = actions_resolved.saturating_add(1);
                steps_succeeded = steps_succeeded.saturating_add(1);
                slots_written = slots_written.saturating_add(1);
            }
            JournalEvent::WaitScheduledEvent { .. } | JournalEvent::AskScheduledEvent { .. } => {
                suspensions = suspensions.saturating_add(1);
            }
            JournalEvent::SlotWrittenEvent { .. } => {
                slots_written = slots_written.saturating_add(1);
            }
            JournalEvent::RunFinished { result, .. } => {
                terminal = Some(RecoveryTerminalState::Finished { result: *result });
            }
            JournalEvent::RunFailedEvent { .. } => terminal = Some(RecoveryTerminalState::Failed),
            JournalEvent::RunCancelled { .. } => terminal = Some(RecoveryTerminalState::Cancelled),
            JournalEvent::RunKilled { .. } => terminal = Some(RecoveryTerminalState::Killed),
            _ => {}
        }
    }
    RecoveryRuntimeSummary {
        run,
        first_seq,
        last_seq,
        workflow,
        steps_started,
        steps_succeeded,
        actions_scheduled,
        actions_resolved,
        suspensions,
        slots_written,
        terminal,
    }
}

fn pending_actions(events: &[JournalEvent]) -> Vec<RecoveredPendingAction> {
    let mut pending: BTreeMap<(StepIdx, ActionId), ()> = BTreeMap::new();
    for event in events {
        match event {
            JournalEvent::ActionScheduled { step, action, .. }
            | JournalEvent::ActionScheduledTicket {
                ticket: vb_core::action::ActionTicket { step, action, .. },
                ..
            } => {
                pending.insert((*step, *action), ());
            }
            JournalEvent::ActionCompletedEvent { step, action, .. }
            | JournalEvent::ActionFailedEvent { step, action, .. }
            | JournalEvent::ActionCompletedEnvelope {
                ticket: vb_core::action::ActionTicket { step, action, .. },
                ..
            }
            | JournalEvent::ActionAbandoned {
                ticket: vb_core::action::ActionTicket { step, action, .. },
                ..
            } => {
                pending.remove(&(*step, *action));
            }
            _ => {}
        }
    }
    pending
        .into_keys()
        .map(|(step, action)| RecoveredPendingAction { step, action })
        .collect()
}
