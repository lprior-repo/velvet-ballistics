use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use crate::value::{SlotValue, Taint, join_taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{CompiledNode, CompiledWorkflow};

use super::super::{ReplayError, engine_to_replay_err};
use super::types::{ReplayAction, SuspensionKind};

pub(super) fn replay_nop(
    node: &CompiledNode,
    run: &mut RunFrame,
) -> Result<ReplayAction, ReplayError> {
    let next = node.routing.next.step().ok_or(ReplayError::Internal {
        reason: "Nop node missing next step",
    })?;
    run.set_pc(next).map_err(engine_to_replay_err)?;
    increment_replay_executed(run)?;
    Ok(ReplayAction::Continue(next))
}

pub(super) fn replay_finish(
    run: &mut RunFrame,
    result: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let _value = *run.read_slot(result).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading finish result slot",
        },
    })?;
    increment_replay_executed(run)?;
    Ok(ReplayAction::Finished)
}

pub(super) fn replay_jump(
    run: &mut RunFrame,
    target: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    run.set_pc(target).map_err(engine_to_replay_err)?;
    increment_replay_executed(run)?;
    Ok(ReplayAction::Continue(target))
}

pub(super) fn replay_suspend(node: &CompiledNode, kind: SuspensionKind) -> ReplayAction {
    ReplayAction::Suspended {
        step: node.id,
        kind,
    }
}

pub(super) fn replay_set_const(
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
    let output = node.output.slot().ok_or(ReplayError::Internal {
        reason: "SetConst node missing output slot",
    })?;
    run.write_slot(output, slot_value)
        .map_err(engine_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

pub(super) fn replay_copy(
    run: &mut RunFrame,
    node: &CompiledNode,
    source: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run.read_slot(source).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading copy source slot",
        },
    })?;
    let taint = run.read_taint(source).map_err(engine_to_replay_err)?;
    let output = node.output.slot().ok_or(ReplayError::Internal {
        reason: "Copy node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(engine_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

pub(super) fn replay_eval_expr(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    expr: ExprIdx,
) -> Result<ReplayAction, ReplayError> {
    let (value, taint) = match crate::engine::eval_expr_with_store(plan, run, store, expr) {
        Ok(result) => result,
        Err(error) if node.routing.error.handler().is_some() => {
            run.mark_failed(node.id).map_err(engine_to_replay_err)?;
            crate::engine::route_error_handler(plan, run, node.id, &error)
                .map_err(engine_to_replay_err)?;
            return Ok(ReplayAction::Continue(run.pc()));
        }
        Err(_) => return Err(ReplayError::ExpressionEvalFailed { step: node.id }),
    };
    let output = node.output.slot().ok_or(ReplayError::Internal {
        reason: "EvalExpr node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(engine_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

pub(super) fn replay_build_object(
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
            EngineError::SlotUninitialized { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_object field slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(engine_to_replay_err)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        entries.push(ObjectField {
            key: *key,
            value,
            taint: slot_taint,
        });
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "build_object field index overflow",
        })?;
    }
    let handle = store
        .insert_object(entries.into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "insert_object failed",
        })?;
    let output = node.output.slot().ok_or(ReplayError::Internal {
        reason: "BuildObject node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::Object(handle), accumulated_taint)
        .map_err(engine_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

pub(super) fn replay_build_list(
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
            EngineError::SlotUninitialized { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_list item slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(engine_to_replay_err)?;
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
    let output = node.output.slot().ok_or(ReplayError::Internal {
        reason: "BuildList node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::List(handle), accumulated_taint)
        .map_err(engine_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

pub(super) fn advance_to_next(
    run: &mut RunFrame,
    node: &CompiledNode,
) -> Result<StepIdx, ReplayError> {
    let next = node.routing.next.step().ok_or(ReplayError::Internal {
        reason: "node missing next step",
    })?;
    run.set_pc(next).map_err(engine_to_replay_err)?;
    increment_replay_executed(run)?;
    Ok(next)
}

pub(super) fn increment_replay_executed(run: &mut RunFrame) -> Result<(), ReplayError> {
    run.increment_executed().map_err(|_| ReplayError::Internal {
        reason: "executed counter overflow",
    })
}
