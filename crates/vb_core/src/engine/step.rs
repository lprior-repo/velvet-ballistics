//! Single-step execution engine.

use super::choose;
use super::expr_eval;
use super::node_helpers;
use super::object_list;
use crate::action::{ActionFailureCode, ActionJournalEvent, ActionTicket};
use crate::ids::ActionId;
use crate::value::Taint;
use crate::EngineSignal;
use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ExprIdx, SlotIdx, StepIdx};
use crate::value::SlotValue;
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
    run.mark_running(pc)?;
    let signal = match execute_node(plan, run, node, store) {
        Ok(signal) => signal,
        Err(error) => {
            run.mark_failed(pc)?;
            return Err(error);
        }
    };
    mark_step_after_signal(run, pc, &signal)?;
    Ok(signal)
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
        other => execute_boundary_node(run, other),
    }
}

#[inline]
fn execute_boundary_node(
    run: &mut RunFrame,
    kind: &CompiledNodeKind,
) -> Result<EngineSignal, EngineError> {
    match kind {
        CompiledNodeKind::Do { .. } => {
            // Engine suspends on Do nodes. The caller receives AwaitingAction
            // and must issue a ticket, dispatch to the action handler, and
            // later call resume_action_completion or resume_action_failure.
            Ok(EngineSignal::AwaitingAction)
        }
        CompiledNodeKind::WaitUntil { .. } | CompiledNodeKind::WaitEvent { .. } => {
            Ok(EngineSignal::AwaitingWait)
        }
        CompiledNodeKind::Ask { .. } => Ok(EngineSignal::AwaitingAsk),
        CompiledNodeKind::Jump { target } => node_helpers::jump_to(run, *target),
        CompiledNodeKind::Finish { result } => node_helpers::finish_run(run, *result),
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
        output_slot,
        output_taint,
    };

    Ok((EngineSignal::Continue, journal))
}

/// Resumes a run after an action failure.
///
/// Marks the suspended step as failed. If the workflow has an error handler
/// for this step (modeled as `ErrorHandler` node kind), the PC is advanced
/// to the handler; otherwise the step remains in the Failed state and the
/// caller receives the failure signal.
///
/// Returns a journal event recording the failure.
///
/// # Errors
///
/// Returns `EngineError` if the step state transition is invalid.
pub fn resume_action_failure(
    run: &mut RunFrame,
    ticket: ActionTicket,
    failure_code: ActionFailureCode,
    retryable: bool,
) -> Result<(EngineSignal, ActionJournalEvent), EngineError> {
    let step = ticket.step;

    // Mark the suspended step as failed.
    run.mark_failed(step)?;

    let journal = ActionJournalEvent::Failed {
        ticket,
        code: failure_code,
        retryable,
    };

    // For now, the failure terminates the step. Error handler routing is
    // a higher-level concern that inspects the journal and node structure.
    Ok((EngineSignal::AwaitingAction, journal))
}

fn mark_step_after_signal(
    run: &mut RunFrame,
    step: StepIdx,
    signal: &EngineSignal,
) -> Result<(), EngineError> {
    match signal {
        EngineSignal::AwaitingWait => run.mark_waiting(step),
        EngineSignal::AwaitingAsk => run.mark_asking(step),
        EngineSignal::AwaitingAction | EngineSignal::StepBudgetExhausted => Ok(()),
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
