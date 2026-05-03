//! Single-step execution engine.

use super::choose;
use super::error_routing::{ErrorHandlerOutcome, route_error_handler};
use super::expr_eval;
use super::node_helpers;
use super::object_list;
use crate::EngineSignal;
use crate::action::{ActionFailureCode, ActionJournalEvent, ActionTicket, RetryPolicy};
use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::ActionId;
use crate::ids::{ExprIdx, SlotIdx, StepIdx};
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
    run.mark_running(pc)?;
    let signal = match execute_node(plan, run, node, store) {
        Ok(signal) => signal,
        Err(error) => {
            run.mark_failed(pc)?;
            // Check if this step has an error handler.
            match route_error_handler(plan, run, pc, &error)? {
                ErrorHandlerOutcome::Routed => {
                    // PC is now at the handler step. Continue execution.
                    return Ok(EngineSignal::Continue);
                }
                ErrorHandlerOutcome::NoHandler => {
                    return Err(error);
                }
            }
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
        CompiledNodeKind::ErrorHandler { body, .. } => {
            // ErrorHandler node routes PC to its body step.
            // If the body fails, the engine's error routing will catch it
            // via the body node's on_error field and route to the handler.
            node_helpers::jump_to(run, *body)
        }
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
        code: failure_code,
        retry_policy,
    };

    // Attempt to route to the error handler if configured.
    let error = EngineError::ResourceLimitExceeded {
        resource: "action_failure",
    };
    match route_error_handler(plan, run, step, &error)? {
        ErrorHandlerOutcome::Routed => Ok((EngineSignal::Continue, journal)),
        ErrorHandlerOutcome::NoHandler => Ok((EngineSignal::AwaitingAction, journal)),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionFailureCode, ActionTicket};
    use crate::frame::StepState;
    use crate::ids::{ActionId, ConstIdx, ExprIdx, RunId, SeqNo, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
    use crate::value::{ConstValue, SlotValue, Taint};
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram,
        ResourceContract, WorkflowParts,
    };

    fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
    where
        T: core::fmt::Debug + PartialEq,
    {
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, found {actual:?}"))
        }
    }

    fn test_frame(workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
        RunFrame::new(
            RunId::new(1),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        )
        .map_err(|e| e.to_string())
    }

    fn nop_then_finish_workflow() -> Result<CompiledWorkflow, String> {
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("nop_finish"),
            digest: WorkflowDigest::from_bytes([0x11; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())
    }

    // ===== Nop dispatch =====

    #[test]
    fn step_once_nop_advances_pc_and_returns_continue() -> Result<(), String> {
        let workflow = nop_then_finish_workflow()?;
        let mut run = test_frame(&workflow)?;
        // Initialize slot 0 so finish succeeds later
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .map_err(|e| e.to_string())?;
        let mut store = ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        ensure_equal(result, EngineSignal::Continue)?;
        ensure_equal(run.pc(), StepIdx::new(1))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))
    }

    // ===== Finish dispatch =====

    #[test]
    fn step_once_finish_returns_finished_with_value_and_taint() -> Result<(), String> {
        let workflow = nop_then_finish_workflow()?;
        let mut run = test_frame(&workflow)?;
        // Advance past the Nop
        run.set_pc(StepIdx::new(1)).map_err(|e| e.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)
            .map_err(|e| e.to_string())?;
        let mut store = ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        ensure_equal(
            result,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Clean),
        )?;
        ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))
    }

    // ===== Do node dispatch =====

    #[test]
    fn step_once_do_returns_awaiting_action() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("do_test"),
            digest: WorkflowDigest::from_bytes([0x22; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(5),
                    input: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        ensure_equal(result, EngineSignal::AwaitingAction)
    }

    // ===== WaitUntil dispatch =====

    #[test]
    fn step_once_wait_returns_awaiting_wait() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("wait_test"),
            digest: WorkflowDigest::from_bytes([0x33; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        ensure_equal(result, EngineSignal::AwaitingWait)?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))
    }

    // ===== Ask dispatch =====

    #[test]
    fn step_once_ask_returns_awaiting_ask() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("ask_test"),
            digest: WorkflowDigest::from_bytes([0x44; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
                    timeout_slot: None,
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        ensure_equal(result, EngineSignal::AwaitingAsk)?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))
    }

    // ===== Jump dispatch =====

    #[test]
    fn step_once_jump_advances_pc_to_target() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("jump_test"),
            digest: WorkflowDigest::from_bytes([0x55; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Jump {
                        target: StepIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        ensure_equal(result, EngineSignal::Continue)?;
        ensure_equal(run.pc(), StepIdx::new(1))
    }

    // ===== EvalExpr dispatch =====

    #[test]
    fn step_once_eval_expr_writes_result_to_output_slot() -> Result<(), String> {
        let expr = ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| crate::WorkflowError::Expression(e))
        .map_err(|e| e.to_string())?;

        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("eval_step_test"),
            digest: WorkflowDigest::from_bytes([0x66; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(19), ConstValue::I64(23)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        ensure_equal(result, EngineSignal::Continue)?;
        ensure_equal(
            *run.read_slot(SlotIdx::new(0)).map_err(|e| e.to_string())?,
            SlotValue::I64(42),
        )
    }

    // ===== BuildObject dispatch =====

    #[test]
    fn step_once_build_object_writes_object_to_output_slot() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("build_obj_step"),
            digest: WorkflowDigest::from_bytes([0x77; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildObject {
                        fields: vec![(SymbolId::new(1), SlotIdx::new(0))].into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(100)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 2,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        // Step 0: SetConst
        let s0 = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
        ensure_equal(s0, EngineSignal::Continue)?;

        // Step 1: BuildObject
        let s1 = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
        ensure_equal(s1, EngineSignal::Continue)?;

        // Verify the output slot contains an Object handle
        match *run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())? {
            SlotValue::Object(_) => Ok(()),
            ref other => Err(format!("expected Object, got {other:?}")),
        }
    }

    // ===== BuildList dispatch =====

    #[test]
    fn step_once_build_list_writes_list_to_output_slot() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("build_list_step"),
            digest: WorkflowDigest::from_bytes([0x88; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildList {
                        items: vec![SlotIdx::new(0)].into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Bool(true)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        // Step 0: SetConst
        let s0 = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
        ensure_equal(s0, EngineSignal::Continue)?;

        // Step 1: BuildList
        let s1 = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
        ensure_equal(s1, EngineSignal::Continue)?;

        match *run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())? {
            SlotValue::List(_) => Ok(()),
            ref other => Err(format!("expected List, got {other:?}")),
        }
    }

    // ===== resume_action_completion =====

    #[test]
    fn resume_action_completion_writes_output_and_advances_pc() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("resume_ok"),
            digest: WorkflowDigest::from_bytes([0xA1; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Do {
                        action: ActionId::new(1),
                        input: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        // Execute the Do node to suspend
        let suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
        ensure_equal(suspend, EngineSignal::AwaitingAction)?;

        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: 0,
        };

        let (signal, _journal) = resume_action_completion(
            &workflow,
            &mut run,
            ticket,
            SlotIdx::new(0),
            SlotValue::I64(99),
            Taint::Clean,
        )
        .map_err(|e| e.to_string())?;

        ensure_equal(signal, EngineSignal::Continue)?;
        ensure_equal(
            *run.read_slot(SlotIdx::new(0)).map_err(|e| e.to_string())?,
            SlotValue::I64(99),
        )?;
        ensure_equal(run.pc(), StepIdx::new(1))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))
    }

    // ===== resume_action_failure =====

    #[test]
    fn resume_action_failure_marks_step_failed() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("resume_fail"),
            digest: WorkflowDigest::from_bytes([0xA2; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        // Execute the Do node to suspend
        let _suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: 0,
        };

        let (signal, _journal) = resume_action_failure(
            &workflow,
            &mut run,
            ticket,
            ActionFailureCode::Timeout,
            true,
        )
        .map_err(|e| e.to_string())?;

        // No error handler, so the signal should be AwaitingAction for external handling
        ensure_equal(signal, EngineSignal::AwaitingAction)?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))
    }

    // ===== journal_action_suspended =====

    #[test]
    fn journal_action_suspended_captures_all_fields() -> Result<(), String> {
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(5),
            attempt: 1,
            idempotency_key: 12345,
        };
        let event = journal_action_suspended(
            ticket,
            ActionId::new(5),
            SlotIdx::new(0),
            SlotIdx::new(1),
            StepIdx::new(0),
        );

        match event {
            crate::action::ActionJournalEvent::Suspended {
                ticket: t,
                action,
                input_slot,
                output_slot,
                step,
            } => {
                ensure_equal(t.run, RunId::new(1))?;
                ensure_equal(action, ActionId::new(5))?;
                ensure_equal(input_slot, SlotIdx::new(0))?;
                ensure_equal(output_slot, SlotIdx::new(1))?;
                ensure_equal(step, StepIdx::new(0))
            }
            other => Err(format!("unexpected event: {other:?}")),
        }
    }

    // ===== ErrorHandler dispatch =====

    #[test]
    fn step_once_error_handler_jumps_to_body() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("error_handler_test"),
            digest: WorkflowDigest::from_bytes([0xB1; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ErrorHandler {
                        body: StepIdx::new(1),
                        handler: StepIdx::new(2),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
            .map_err(|e| e.to_string())?;
        let mut store = ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        // ErrorHandler should jump to its body step
        ensure_equal(result, EngineSignal::Continue)?;
        ensure_equal(run.pc(), StepIdx::new(1))
    }
}
