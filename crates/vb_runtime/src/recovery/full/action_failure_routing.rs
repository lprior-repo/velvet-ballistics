#![forbid(unsafe_code)]
//! Restores an action failure's handler routing from partial journal evidence.
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
//! restorable without new journal bytes. Two durable shapes are repaired: a
//! failure with nothing after it (the jump was lost), and a failure followed by
//! exactly the handler's `StepStarted` (the jump happened, the handler effect
//! did not). Every other shape stays untouched: retryable failures carry
//! `RetryTimerScheduled`, unhandled failures are terminal through `RunFailed`,
//! and further progress means the routing decision is already durable.
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

/// Journaled evidence that follows the routed action failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureEvidence {
    /// Nothing follows the failure: the handler jump was lost.
    Unjournaled,
    /// The handler (and, for a wrapper, its jump target) was started.
    Started {
        first: StepIdx,
        second: Option<StepIdx>,
    },
}

/// Action failure whose handler transition is only partially journaled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RoutedFailure {
    step: StepIdx,
    action: ActionId,
    evidence: FailureEvidence,
}

/// Handler route the live failure path resolves for a failed step.
///
/// `handler` is exactly what [`find_error_handler_for_failure`] returned and
/// what the live preflight jumped to. When that node is a legacy
/// `ErrorHandler`, the drive executes its `body` next, so `handler_body`
/// records the engine's continuation step; `error_slot` is the route's
/// authored failure slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HandlerRoute {
    pub(crate) handler: StepIdx,
    pub(crate) handler_body: Option<StepIdx>,
    pub(crate) error_slot: Option<SlotIdx>,
}

/// Resolves the full handler route for `failed`, including the legacy
/// `ErrorHandler` body the drive executes after the resolved handler, without
/// broadening handler authority.
pub(crate) fn handler_route_for_failure(
    workflow: &CompiledWorkflow,
    failed: StepIdx,
) -> Option<HandlerRoute> {
    let (handler, error_slot) = find_error_handler_for_failure(workflow, failed)?;
    let handler_body = match workflow.node(handler)?.kind {
        CompiledNodeKind::ErrorHandler { body, .. } => Some(body),
        _ => None,
    };
    Some(HandlerRoute {
        handler,
        handler_body,
        error_slot,
    })
}

/// Restores the journal-authorized handler route of a failed action without
/// re-driving the resolved `Do` step or re-emitting durable step evidence.
///
/// Reuses [`find_error_handler_for_failure`] so recovery resolves the handler
/// exactly as the failure path did, including legacy error-handler wrappers.
pub(crate) fn restore_routed_action_failure(
    seed: &mut RecoveryFrameSeed,
    events: &[JournalEvent],
    workflow: &CompiledWorkflow,
) -> RecoveryResult<()> {
    let Some(failed) = routed_failure_evidence(events) else {
        return Ok(());
    };
    let Some(route) = restorable_route(seed, &failed, workflow) else {
        return Ok(());
    };
    match failed.evidence {
        FailureEvidence::Unjournaled => {
            if seed.pc == failed.step {
                restore_transition(seed, &failed, route.handler, &route);
            }
        }
        FailureEvidence::Started { first, second } => {
            if let Some(pc) = resumed_step(first, second, &route) {
                restore_transition(seed, &failed, pc, &route);
            }
        }
    }
    Ok(())
}

/// Guards the repair: the seed must still be parked on the resolved `Do` step
/// in `Running` state, and the compiled workflow must route that failure.
fn restorable_route(
    seed: &RecoveryFrameSeed,
    failed: &RoutedFailure,
    workflow: &CompiledWorkflow,
) -> Option<HandlerRoute> {
    if recovered_step_state(seed, failed.step) != Some(RecoveredStepState::Running) {
        return None;
    }
    let node = workflow.node(failed.step)?;
    if !matches!(node.kind, CompiledNodeKind::Do { action, .. } if action == failed.action) {
        return None;
    }
    handler_route_for_failure(workflow, failed.step)
}

/// Program counter implied by the journaled start chain, when the chain is
/// exactly the resolved handler (and, for a wrapper, its jump target).
fn resumed_step(first: StepIdx, second: Option<StepIdx>, route: &HandlerRoute) -> Option<StepIdx> {
    if first != route.handler {
        return None;
    }
    match (second, route.handler_body) {
        (None, _) => Some(first),
        (Some(second), Some(inner)) if second == inner => Some(second),
        _ => None,
    }
}

/// Applies the recovered routing decision: failed step, program counter, and
/// the error slot the live preflight wrote before driving the handler.
fn restore_transition(
    seed: &mut RecoveryFrameSeed,
    failed: &RoutedFailure,
    pc: StepIdx,
    route: &HandlerRoute,
) {
    mark_failed(seed, failed.step);
    seed.pc = pc;
    if let Some(slot) = route.error_slot {
        record_error_slot(seed, slot, failed.step);
    }
}

/// Returns the last `ActionFailed` plus the evidence that follows it. Only an
/// empty or a start chain along the resolved handler is recoverable; any other
/// later event proves the routing decision is already durable.
fn routed_failure_evidence(events: &[JournalEvent]) -> Option<RoutedFailure> {
    let (index, step, action) = last_action_failed(events)?;
    let evidence = match events.get(index.checked_add(1)?..)? {
        [] => FailureEvidence::Unjournaled,
        [JournalEvent::StepStarted { step: first, .. }] => FailureEvidence::Started {
            first: *first,
            second: None,
        },
        [
            JournalEvent::StepStarted { step: first, .. },
            JournalEvent::StepStarted { step: second, .. },
        ] => FailureEvidence::Started {
            first: *first,
            second: Some(*second),
        },
        _ => return None,
    };
    Some(RoutedFailure {
        step,
        action,
        evidence,
    })
}

fn last_action_failed(events: &[JournalEvent]) -> Option<(usize, StepIdx, ActionId)> {
    events
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, event)| match event {
            JournalEvent::ActionFailedEvent { step, action, .. } => Some((index, *step, *action)),
            _ => None,
        })
}

fn recovered_step_state(seed: &RecoveryFrameSeed, step: StepIdx) -> Option<RecoveredStepState> {
    seed.steps
        .iter()
        .find(|entry| entry.step == step)
        .map(|entry| entry.state)
}

fn mark_failed(seed: &mut RecoveryFrameSeed, step: StepIdx) {
    if let Some(entry) = seed.steps.iter_mut().find(|entry| entry.step == step) {
        entry.state = RecoveredStepState::Failed;
    }
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
