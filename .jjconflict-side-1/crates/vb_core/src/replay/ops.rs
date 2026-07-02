#![forbid(unsafe_code)]
//! Expression evaluation operations for replay.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{AccessorIdx, ConstIdx, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint, join_taint};
use crate::value_store::ValueStore;
use crate::workflow::{CompiledWorkflow, ExprOp};

use super::{ReplayError, ReplayExprStack};

pub fn eval_replay_op(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &mut ValueStore,
    op: ExprOp,
    stack: &mut ReplayExprStack,
    taint_accum: &mut Taint,
) -> Result<(), ReplayError> {
    match op {
        ExprOp::LoadSlot(slot) => eval_load_slot(run, slot, stack, taint_accum),
        ExprOp::LoadConst(constant) => eval_load_const(plan, constant, stack),
        ExprOp::LoadAccessor(accessor) => {
            eval_load_accessor(plan, run, store, accessor, stack, taint_accum)
        }
        ExprOp::Eq => eval_eq(stack),
        ExprOp::NotEq => eval_not_eq(stack),
        ExprOp::And => eval_and(stack),
        ExprOp::Or => eval_or(stack),
        ExprOp::Not => eval_not(stack),
        ExprOp::Add => eval_add(stack),
        ExprOp::Sub => eval_sub(stack),
        ExprOp::Mul => eval_mul(stack),
        ExprOp::Div => eval_div(stack),
        ExprOp::Gt => eval_gt(stack),
        ExprOp::Gte => eval_gte(stack),
        ExprOp::Lt => eval_lt(stack),
        ExprOp::Lte => eval_lte(stack),
        _ => Err(ReplayError::Internal {
            reason: "unsupported expression op for replay",
        }),
    }
}

fn eval_load_slot(
    run: &RunFrame,
    slot: SlotIdx,
    stack: &mut ReplayExprStack,
    taint_accum: &mut Taint,
) -> Result<(), ReplayError> {
    let value = *run.read_slot(slot).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
        EngineError::SlotUninitialized { slot: s } => ReplayError::SlotNotAvailable { slot: s },
        _ => ReplayError::Internal {
            reason: "unexpected error reading expression load slot",
        },
    })?;
    let slot_taint = run.read_taint(slot).map_err(|_| ReplayError::Internal {
        reason: "read_taint failed",
    })?;
    *taint_accum = join_taint(*taint_accum, slot_taint);
    stack.push(value)
}

fn eval_load_const(
    plan: &CompiledWorkflow,
    constant: ConstIdx,
    stack: &mut ReplayExprStack,
) -> Result<(), ReplayError> {
    let value = plan
        .constant(constant)
        .ok_or(ReplayError::Internal {
            reason: "constant out of bounds",
        })?
        .to_slot_value()
        .map_err(|_| ReplayError::Internal {
            reason: "constant to slot value failed",
        })?;
    stack.push(value)
}

fn eval_load_accessor(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &mut ValueStore,
    accessor: AccessorIdx,
    stack: &mut ReplayExprStack,
    taint_accum: &mut Taint,
) -> Result<(), ReplayError> {
    let accessor_program = plan.accessor(accessor).ok_or(ReplayError::Internal {
        reason: "accessor out of bounds",
    })?;
    let (value, accessor_taint) = eval_accessor_for_replay(run, store, accessor_program)?;
    *taint_accum = join_taint(*taint_accum, accessor_taint);
    stack.push(value)
}

fn eval_eq(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    stack.push(SlotValue::Bool(left == right))
}

fn eval_not_eq(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    stack.push(SlotValue::Bool(left != right))
}

fn eval_and(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    let left_bool = expect_bool_replay(left)?;
    let right_bool = expect_bool_replay(right)?;
    stack.push(SlotValue::Bool(left_bool && right_bool))
}

fn eval_or(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    let left_bool = expect_bool_replay(left)?;
    let right_bool = expect_bool_replay(right)?;
    stack.push(SlotValue::Bool(left_bool || right_bool))
}

fn eval_not(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let b = expect_bool_replay(value)?;
    stack.push(SlotValue::Bool(!b))
}

pub(crate) fn eval_add(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_add(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

pub(crate) fn eval_sub(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_sub(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

pub(crate) fn eval_mul(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_mul(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

fn eval_div(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_div(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

fn eval_gt(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left > right))
}

fn eval_gte(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left >= right))
}

fn eval_lt(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left < right))
}

fn eval_lte(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left <= right))
}

fn eval_accessor_for_replay(
    run: &RunFrame,
    store: &mut ValueStore,
    program: &crate::workflow::AccessorProgram,
) -> Result<(SlotValue, Taint), ReplayError> {
    let mut current = *run.read_slot(program.root).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading accessor root",
        },
    })?;
    let mut accumulated_taint =
        run.read_taint(program.root)
            .map_err(|_| ReplayError::Internal {
                reason: "read_taint failed for accessor root",
            })?;
    if program.path.is_empty() {
        return Ok((current, accumulated_taint));
    }
    let mut index = 0usize;
    while index < program.path.len() {
        let segment = program
            .path
            .get(index)
            .copied()
            .ok_or(ReplayError::Internal {
                reason: "accessor path index checked by loop bound",
            })?;
        let (next_value, segment_taint) = match (current, segment) {
            (SlotValue::Object(object), crate::workflow::PathSegment::Field(field)) => store
                .object_field_with_taint(object, field)
                .map_err(|_| ReplayError::Internal {
                    reason: "object field not found during replay accessor",
                })?,
            (SlotValue::List(list), crate::workflow::PathSegment::Index(idx)) => store
                .list_item_with_taint(list, idx)
                .map_err(|_| ReplayError::Internal {
                    reason: "list index out of bounds during replay accessor",
                })?,
            (_, _) => {
                return Err(ReplayError::Internal {
                    reason: "unsupported accessor traversal during replay",
                });
            }
        };
        accumulated_taint = join_taint(accumulated_taint, segment_taint);
        current = next_value;
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "accessor path index overflow",
        })?;
    }
    Ok((current, accumulated_taint))
}

pub fn pop_pair(stack: &mut ReplayExprStack) -> Result<(SlotValue, SlotValue), ReplayError> {
    let right = stack.pop()?;
    let left = stack.pop()?;
    Ok((left, right))
}

pub fn pop_i64_pair(stack: &mut ReplayExprStack) -> Result<(i64, i64), ReplayError> {
    let right = stack.pop()?;
    let left = stack.pop()?;
    Ok((expect_i64_replay(left)?, expect_i64_replay(right)?))
}

fn expect_bool_replay(value: SlotValue) -> Result<bool, ReplayError> {
    match value {
        SlotValue::Bool(b) => Ok(b),
        _ => Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        }),
    }
}

fn expect_i64_replay(value: SlotValue) -> Result<i64, ReplayError> {
    match value {
        SlotValue::I64(v) => Ok(v),
        _ => Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        }),
    }
}

#[cfg(test)]
#[path = "ops/tests.rs"]
mod tests;
