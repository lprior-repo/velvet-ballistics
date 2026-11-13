#![forbid(unsafe_code)]
//! Restores an action failure's handler jump from partial journal evidence.
//!
//! `Shard::handle_action_failure` appends `ActionFailed` and then performs the
//! handler transition — `mark_failed`, the error-slot write, and the program
//! counter jump — in memory before driving the handler, which journals
//! `StepStarted(handler)`. A crash inside that window leaves durable evidence
//! that resolves the action but not the routing decision, so replayed state is
//! still positioned on the resolved `Do` step and marked `Running`. Driving
//! that frame mints a fresh ticket for an action that already ran, which is a
//! reissued external effect.
//!
//! The routing target and the error slot are static properties of the compiled
//! workflow, and the failed step index is journaled, so the transition is
//! restorable without new journal bytes. Every other shape stays untouched:
//! retryable failures carry `RetryTimerScheduled`, unhandled failures are
//! terminal through `RunFailed`, and any later progress event (including an
//! already-started handler) means the routing decision is already durable.
//!
//! Only the workflow-aware replay can perform this repair. The workflow-less
//! `recover_runtime_frame_seed_from_events` entry point has no handler table,
//! so its product keeps the typed `CannotResume` witness and can never
//! authorize a live resume or a silent re-drive.

use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};
use vb_core::{ActionId, SlotIdx, SlotValue, StepIdx, Taint};
use vb_storage::JournalEvent;
use vb_storage::recovery::{
    RecoveredSlotEntry, RecoveredStepState, RecoveryFrameSeed, RecoveryResult,
};

use crate::shard::helpers::find_error_handler_for_failure;

/// Action failure whose handler transition was not journaled yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RoutedFailure {
    step: StepIdx,
    action: ActionId,
}

/// Rewrites `seed` into the live post-routing state when the journal ends on an
/// unrouted action failure for the step the seed is positioned at.
///
/// Reuses [`find_error_handler_for_failure`] so recovery resolves the handler
/// exactly as the failure path did, including legacy error-handler wrappers.
pub(crate) fn restore_routed_action_failure(
    seed: &mut RecoveryFrameSeed,
    events: &[JournalEvent],
    workflow: &CompiledWorkflow,
) -> RecoveryResult<()> {
    let Some(failed) = last_unrouted_action_failure(events) else {
        return Ok(());
    };
    if seed.pc != failed.step {
        return Ok(());
    }
    if recovered_step_state(seed, failed.step) != Some(RecoveredStepState::Running) {
        return Ok(());
    }
    let Some(node) = workflow.node(failed.step) else {
        return Ok(());
    };
    if !matches!(node.kind, CompiledNodeKind::Do { action, .. } if action == failed.action) {
        return Ok(());
    }
    let Some((handler, error_slot)) = find_error_handler_for_failure(workflow, failed.step) else {
        return Ok(());
    };
    let Some(entry) = seed
        .steps
        .iter_mut()
        .find(|entry| entry.step == failed.step)
    else {
        return Ok(());
    };
    entry.state = RecoveredStepState::Failed;
    seed.pc = handler;
    if let Some(slot) = error_slot {
        record_error_slot(seed, slot, failed.step);
    }
    Ok(())
}

/// Returns the trailing `ActionFailed` only when no later progress event
/// exists. Any step, action, retry, suspension, or terminal event after the
/// failure proves the routing decision (or a longer prefix) is already
/// journaled, so nothing is inferred.
fn last_unrouted_action_failure(events: &[JournalEvent]) -> Option<RoutedFailure> {
    let (index, step, action) =
        events
            .iter()
            .enumerate()
            .fold(None, |pending, (index, event)| match event {
                JournalEvent::ActionFailedEvent { step, action, .. } => {
                    Some((index, *step, *action))
                }
                JournalEvent::StepStarted { .. }
                | JournalEvent::StepSucceeded { .. }
                | JournalEvent::StepFailed { .. }
                | JournalEvent::ActionScheduled { .. }
                | JournalEvent::ActionScheduledTicket { .. }
                | JournalEvent::ActionCompletedEvent { .. }
                | JournalEvent::ActionCompletedEnvelope { .. }
                | JournalEvent::ActionAbandoned { .. }
                | JournalEvent::RetryCheckpoint { .. }
                | JournalEvent::RetryTimerScheduled { .. }
                | JournalEvent::RunFinished { .. }
                | JournalEvent::RunFailedEvent { .. }
                | JournalEvent::RunCancelled { .. }
                | JournalEvent::RunKilled { .. } => None,
                _ => pending,
            })?;
    if events.get(index.checked_add(1)?..)?.is_empty() {
        Some(RoutedFailure { step, action })
    } else {
        None
    }
}

fn recovered_step_state(seed: &RecoveryFrameSeed, step: StepIdx) -> Option<RecoveredStepState> {
    seed.steps
        .iter()
        .find(|entry| entry.step == step)
        .map(|entry| entry.state)
}

/// Writes `I64(failed_step)` with `Clean` taint, matching `RunFrame::write_slot`
/// on the live failure path.
fn record_error_slot(seed: &mut RecoveryFrameSeed, slot: SlotIdx, step: StepIdx) {
    let entry = RecoveredSlotEntry {
        slot,
        value: SlotValue::I64(i64::from(step.get())),
        taint: Taint::Clean,
    };
    match seed.slots.iter_mut().find(|existing| existing.slot == slot) {
        Some(existing) => *existing = entry,
        None => seed.slots.push(entry),
    }
}
