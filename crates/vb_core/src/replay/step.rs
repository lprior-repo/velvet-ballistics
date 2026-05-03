//! Replay step execution.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use crate::value::{join_taint, SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use super::{eval_expr_for_replay, slot_to_replay_err, ReplayError, ReplayExprStack};

/// Internal action returned by `replay_step`.
pub enum ReplayAction {
    /// Continue to the next step.
    Continue(StepIdx),
    /// The run finished.
    Finished,
    /// The run is suspended on a non-deterministic node.
    Suspended { step: StepIdx, kind: &'static str },
}

/// Replays a single deterministic step.
///
/// For deterministic node kinds (SetConst, Copy, EvalExpr, BuildObject, BuildList,
/// Finish, Nop), executes the same logic as the engine's `step_once`.
/// For non-deterministic (Do/Action, Ask, WaitUntil, WaitEvent), returns a
/// suspension signal.
pub fn replay_step(
    node: &CompiledNode,
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: &CompiledWorkflow,
) -> Result<ReplayAction, ReplayError> {
    match &node.kind {
        CompiledNodeKind::Nop => replay_nop(node, run),
        CompiledNodeKind::SetConst { value } => replay_set_const(plan, run, node, *value),
        CompiledNodeKind::Copy { source } => replay_copy(run, node, *source),
        CompiledNodeKind::EvalExpr { expr } => replay_eval_expr(plan, run, store, node, *expr),
        CompiledNodeKind::BuildObject { fields } => replay_build_object(run, store, node, fields),
        CompiledNodeKind::BuildList { items } => replay_build_list(run, store, node, items),
        CompiledNodeKind::Finish { result } => replay_finish(run, *result),
        CompiledNodeKind::Jump { target } => replay_jump(run, *target),
        CompiledNodeKind::Do { .. } => replay_suspend(node, "Do"),
        CompiledNodeKind::Ask { .. } => replay_suspend(node, "Ask"),
        CompiledNodeKind::WaitUntil { .. } => replay_suspend(node, "WaitUntil"),
        CompiledNodeKind::WaitEvent { .. } => replay_suspend(node, "WaitEvent"),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => super::choose::replay_choose_slot(run, branches, *otherwise),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => super::choose::replay_choose_expr(plan, run, store, branches, *otherwise),
        _ => Err(ReplayError::Internal {
            reason: "unsupported node kind for replay",
        }),
    }
}

fn replay_nop(node: &CompiledNode, run: &mut RunFrame) -> Result<ReplayAction, ReplayError> {
    let next = node.next.ok_or(ReplayError::Internal {
        reason: "Nop node missing next step",
    })?;
    run.set_pc(next).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(next))
}

fn replay_finish(run: &mut RunFrame, result: SlotIdx) -> Result<ReplayAction, ReplayError> {
    let _value = *run.read_slot(result).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading finish result slot",
        },
    })?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Finished)
}

fn replay_jump(run: &mut RunFrame, target: StepIdx) -> Result<ReplayAction, ReplayError> {
    run.set_pc(target).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(target))
}

fn replay_suspend(node: &CompiledNode, kind: &'static str) -> Result<ReplayAction, ReplayError> {
    Ok(ReplayAction::Suspended {
        step: node.id,
        kind,
    })
}

fn replay_set_const(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    node: &CompiledNode,
    value: ConstIdx,
) -> Result<ReplayAction, ReplayError> {
    let constant = plan.constant(value).copied().ok_or(ReplayError::Internal {
        reason: "constant out of bounds",
    })?;
    let slot_value = constant
        .to_slot_value()
        .map_err(|_| ReplayError::Internal {
            reason: "constant to slot value failed",
        })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "SetConst node missing output slot",
    })?;
    run.write_slot(output, slot_value)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_copy(
    run: &mut RunFrame,
    node: &CompiledNode,
    source: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run.read_slot(source).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading copy source slot",
        },
    })?;
    let taint = run.read_taint(source).map_err(slot_to_replay_err)?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "Copy node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_eval_expr(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    expr: ExprIdx,
) -> Result<ReplayAction, ReplayError> {
    let (value, taint) = eval_expr_for_replay(plan, run, store, expr)
        .map_err(|_| ReplayError::ExpressionEvalFailed { step: node.id })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "EvalExpr node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_build_object(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    fields: &[(SymbolId, SlotIdx)],
) -> Result<ReplayAction, ReplayError> {
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(fields.len())
        .map_err(|_| ReplayError::Internal {
            reason: "allocation failed",
        })?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < fields.len() {
        let (key, slot) = fields.get(index).ok_or(ReplayError::Internal {
            reason: "build_object field index checked by loop bound",
        })?;
        let value = *run.read_slot(*slot).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_object field slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(slot_to_replay_err)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        entries.push(ObjectField { key: *key, value });
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "build_object field index overflow",
        })?;
    }
    let handle = store
        .insert_object(entries.into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "insert_object failed",
        })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "BuildObject node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::Object(handle), accumulated_taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_build_list(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    items: &[SlotIdx],
) -> Result<ReplayAction, ReplayError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(items.len())
        .map_err(|_| ReplayError::Internal {
            reason: "allocation failed",
        })?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < items.len() {
        let slot = items.get(index).ok_or(ReplayError::Internal {
            reason: "build_list item index checked by loop bound",
        })?;
        let value = *run.read_slot(*slot).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_list item slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(slot_to_replay_err)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        values.push(value);
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "build_list item index overflow",
        })?;
    }
    let handle =
        store
            .insert_list(values.into_boxed_slice())
            .map_err(|_| ReplayError::Internal {
                reason: "insert_list failed",
            })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "BuildList node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::List(handle), accumulated_taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn advance_to_next(run: &mut RunFrame, node: &CompiledNode) -> Result<StepIdx, ReplayError> {
    let next = node.next.ok_or(ReplayError::Internal {
        reason: "node missing next step",
    })?;
    run.set_pc(next).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(next)
}
