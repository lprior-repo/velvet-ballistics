#![forbid(unsafe_code)]
//! Single-step execution engine.

use super::choose;
use super::error_routing::{ErrorHandlerOutcome, route_error_handler};
use super::expr_eval;
use super::node_helpers;
use super::object_list;
use crate::EngineSignal;
use crate::action::{ActionFailureCode, ActionJournalEvent, ActionTicket, RetryPolicy};
use crate::errors::EngineError;
use crate::frame::{RunFrame, StepState};
use crate::ids::ActionId;
use crate::ids::{ExprIdx, SeqNo, SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::value::Taint;
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNodeKind, CompiledWorkflow};

pub fn step_once(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    let pc = run.pc();
    let node = plan
        .node(pc)
        .ok_or(EngineError::InvalidProgramCounter { step: pc })?;

    // Loop body re-entry: when a step is already Succeeded, preserve it.
    let is_reentry = matches!(run.step_state(pc)?, StepState::Succeeded);
    if !is_reentry {
        run.mark_running(pc)?;
    }

    let signal = match execute_node(plan, run, node, store) {
        Ok(signal) => signal,
        Err(error) => return handle_step_error(plan, run, pc, is_reentry, error),
    };

    if !is_reentry {
        mark_step_after_signal(run, pc, &signal)?;
    }
    Ok(signal)
}

#[inline]
fn handle_step_error(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    pc: StepIdx,
    is_reentry: bool,
    error: EngineError,
) -> Result<EngineSignal, EngineError> {
    // On re-entry, the step stays in its terminal state.
    if !is_reentry {
        run.mark_failed(pc)?;
    }
    match route_error_handler(plan, run, pc, &error)? {
        ErrorHandlerOutcome::Routed => Ok(EngineSignal::Continue),
        ErrorHandlerOutcome::NoHandler => Err(error),
    }
}

#[inline]
fn execute_node(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    match &node.kind {
        CompiledNodeKind::Nop => node_helpers::jump_to_next(run, node.next, node.id),
        CompiledNodeKind::SetConst { value } => node_helpers::set_const(plan, run, node, *value),
        CompiledNodeKind::Copy { source } => node_helpers::copy_slot(run, node, *source),
        CompiledNodeKind::EvalExpr { expr } => eval_expr_node(plan, run, node, store, *expr),
        CompiledNodeKind::BuildObject { fields } => build_object_node(run, node, fields, store),
        CompiledNodeKind::BuildList { items } => build_list_node(run, node, items, store),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => choose::choose_slot_branch(run, branches, *otherwise),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => choose::choose_expr_branch(plan, run, store, branches, *otherwise),
        other => execute_boundary_node(run, node.id, other),
    }
}

#[inline]
fn execute_boundary_node(
    run: &mut RunFrame,
    step: StepIdx,
    kind: &CompiledNodeKind,
) -> Result<EngineSignal, EngineError> {
    match kind {
        CompiledNodeKind::Jump { target } => node_helpers::jump_to(run, *target),
        CompiledNodeKind::Finish { result } => node_helpers::finish_run(run, *result),
        CompiledNodeKind::ErrorHandler { body, .. } => node_helpers::jump_to(run, *body),
        other => execute_suspension_node(step, other),
    }
}

#[inline]
fn execute_suspension_node(
    step: StepIdx,
    kind: &CompiledNodeKind,
) -> Result<EngineSignal, EngineError> {
    match kind {
        CompiledNodeKind::Do { action, .. } => Ok(EngineSignal::AwaitingAction {
            step,
            // The IR interpreter does not track journal sequence numbers;
            // it emits ZERO as a sentinel. The runtime resolves the real
            // sequence number from its journal sequence tracker before
            // constructing the live ActionTicket.
            seq: SeqNo::ZERO,
            action: *action,
        }),
        CompiledNodeKind::WaitUntil { deadline_slot } => Ok(EngineSignal::AwaitingWait {
            deadline_slot: *deadline_slot,
        }),
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => Ok(EngineSignal::AwaitingEvent {
            event: *event,
            timeout_slot: *timeout_slot,
        }),
        CompiledNodeKind::Ask { timeout_slot, .. } => Ok(EngineSignal::AwaitingAsk {
            timeout_slot: *timeout_slot,
        }),
        _ => Err(EngineError::UnsupportedPrimitive {
            primitive: "not_yet_implemented",
        }),
    }
}

/// Constructs a journal event for Do-node suspension.
///
/// Callers should record this event before acknowledging the suspension.
/// The event captures the ticket, action ID, input/output slots, and step.
pub fn journal_action_suspended(
    ticket: ActionTicket,
    action: ActionId,
    input_slot: SlotIdx,
    output_slot: SlotIdx,
    step: StepIdx,
) -> ActionJournalEvent {
    ActionJournalEvent::Suspended {
        ticket,
        attempt: ticket.attempt,
        action,
        input_slot,
        output_slot,
        step,
    }
}

/// Resumes a run after a successful action completion.
///
/// Writes the output value and taint to the designated slot, marks the
/// suspended step as succeeded, and advances the PC to the next step.
/// Returns a journal event recording the completion.
///
/// # Errors
///
/// Returns `EngineError` if the output slot write fails, the step state
/// transition is invalid, or the next step is missing.
pub fn resume_action_completion(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    ticket: ActionTicket,
    output_slot: SlotIdx,
    output_value: SlotValue,
    output_taint: Taint,
) -> Result<(EngineSignal, ActionJournalEvent), EngineError> {
    let step = ticket.step;
    let next = plan
        .node(step)
        .ok_or(EngineError::InvalidProgramCounter { step })?
        .next
        .ok_or(EngineError::MissingNextStep { step })?;

    // Write the action output to the designated slot.
    run.write_slot_with_taint(output_slot, output_value, output_taint)?;

    // Mark the suspended step as succeeded.
    run.mark_succeeded(step)?;

    // Advance the program counter past the Do node.
    run.set_pc(next)?;
    run.increment_executed()?;

    let journal = ActionJournalEvent::Completed {
        ticket,
        attempt: ticket.attempt,
        output_slot,
        output_taint,
    };

    Ok((EngineSignal::Continue, journal))
}

/// Resumes a run after an action failure.
///
/// Marks the suspended step as failed. If the workflow has an error handler
/// for this step, the PC is advanced to the handler and the engine signal
/// is `Continue`; otherwise the step remains in the Failed state and the
/// caller receives `AwaitingAction` to handle the failure externally.
///
/// Returns a journal event recording the failure.
///
/// # Errors
///
/// Returns `EngineError` if the step state transition is invalid.
pub fn resume_action_failure(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    ticket: ActionTicket,
    failure_code: ActionFailureCode,
    retry_policy: RetryPolicy,
) -> Result<(EngineSignal, ActionJournalEvent), EngineError> {
    let step = ticket.step;

    // Mark the suspended step as failed.
    run.mark_failed(step)?;

    let journal = ActionJournalEvent::Failed {
        ticket,
        attempt: ticket.attempt,
        code: failure_code,
        retry_policy,
    };

    // Attempt to route to the error handler if configured.
    let error = EngineError::ResourceLimitExceeded {
        resource: "action_failure",
    };
    match route_error_handler(plan, run, step, &error)? {
        ErrorHandlerOutcome::Routed => Ok((EngineSignal::Continue, journal)),
        ErrorHandlerOutcome::NoHandler => Ok((
            EngineSignal::AwaitingAction {
                step: ticket.step,
                // No journal sequence bump on handler routing failure;
                // the runtime resolves seq from its tracker.
                seq: SeqNo::ZERO,
                action: ticket.action,
            },
            journal,
        )),
    }
}

fn mark_step_after_signal(
    run: &mut RunFrame,
    step: StepIdx,
    signal: &EngineSignal,
) -> Result<(), EngineError> {
    match signal {
        EngineSignal::AwaitingWait { .. } | EngineSignal::AwaitingEvent { .. } => {
            run.mark_waiting(step)
        }
        EngineSignal::AwaitingAsk { .. } => run.mark_asking(step),
        EngineSignal::AwaitingAction { .. } | EngineSignal::StepBudgetExhausted => Ok(()),
        EngineSignal::Continue | EngineSignal::Finished(_, _) => run.mark_succeeded(step),
    }
}

#[inline]
fn eval_expr_node(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    store: &mut ValueStore,
    expr: ExprIdx,
) -> Result<EngineSignal, EngineError> {
    let (value, taint) = expr_eval::eval_expr_with_store(plan, run, store, expr)?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot_with_taint(output, value, taint)?;
    node_helpers::jump_to_next(run, node.next, node.id)
}

#[inline]
fn build_object_node(
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    fields: &[(crate::ids::SymbolId, SlotIdx)],
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    let (handle, taint) = object_list::build_object_with_taint(store, run, fields)?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot_with_taint(output, SlotValue::Object(handle), taint)?;
    node_helpers::jump_to_next(run, node.next, node.id)
}

#[inline]
fn build_list_node(
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    items: &[SlotIdx],
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    let (handle, taint) = object_list::build_list_with_taint(store, run, items)?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot_with_taint(output, SlotValue::List(handle), taint)?;
    node_helpers::jump_to_next(run, node.next, node.id)
}

#[cfg(test)]
#[path = "step/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "step/edge_cases.rs"]
mod edge_cases;

#[cfg(test)]
#[path = "step/resume_failure.rs"]
mod resume_failure;

#[cfg(test)]
#[path = "step/choose_branch.rs"]
mod choose_branch;

#[cfg(test)]
#[path = "step/node_helpers_err.rs"]
mod node_helpers_err;
