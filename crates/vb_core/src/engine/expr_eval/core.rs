//! Expression evaluation core.

use crate::errors::EngineError;
use crate::ids::{ConstIdx, ExprIdx, SlotIdx};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{CompiledWorkflow, ExprOp};

use super::accessors::eval_load_accessor;
use super::ops::eval_expr_operator;
use super::stack::{ExprStack, push_value};

fn expression_op(ops: &[ExprOp], index: usize) -> Result<ExprOp, EngineError> {
    ops.get(index)
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "expression op index checked by loop bound",
        })
}

fn next_expr_index(index: usize) -> Result<usize, EngineError> {
    index
        .checked_add(1)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "expression op index overflow",
        })
}

fn finish_expr_stack(stack: &mut ExprStack) -> Result<SlotValue, EngineError> {
    if stack.len() == 1 {
        stack.pop()
    } else {
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "expression leaves non-single result",
        })
    }
}

pub(super) fn eval_expr_inner(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    expr: ExprIdx,
) -> Result<(SlotValue, Taint), EngineError> {
    let program = plan
        .expression(expr)
        .ok_or(EngineError::ExprOutOfBounds { expr })?;
    let mut stack = ExprStack::new(program.max_stack)?;
    let mut taint_accum = Taint::Clean;
    let mut index = 0usize;
    while index < program.ops.len() {
        let op = expression_op(program.ops.as_ref(), index)?;
        eval_expr_op(plan, run, store, op, &mut stack, &mut taint_accum)?;
        index = next_expr_index(index)?;
    }
    let value = finish_expr_stack(&mut stack)?;
    Ok((value, taint_accum))
}

fn eval_load_slot(
    run: &crate::RunFrame,
    stack: &mut ExprStack,
    slot: SlotIdx,
    taint_accum: &mut Taint,
) -> Result<(), EngineError> {
    let value = *run.read_slot(slot)?;
    let slot_taint = run.read_taint(slot)?;
    *taint_accum = crate::value::join_taint(*taint_accum, slot_taint);
    push_value(stack, value)
}

fn eval_load_const(
    plan: &CompiledWorkflow,
    stack: &mut ExprStack,
    constant: ConstIdx,
) -> Result<(), EngineError> {
    push_value(
        stack,
        plan.constant(constant)
            .ok_or(EngineError::ConstOutOfBounds { index: constant })?
            .to_slot_value()?,
    )
}

pub(super) fn eval_expr_op(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    op: ExprOp,
    stack: &mut ExprStack,
    taint_accum: &mut Taint,
) -> Result<(), EngineError> {
    match op {
        ExprOp::LoadSlot(slot) => eval_load_slot(run, stack, slot, taint_accum),
        ExprOp::LoadConst(constant) => eval_load_const(plan, stack, constant),
        ExprOp::LoadAccessor(accessor) => {
            eval_load_accessor(plan, run, store, stack, accessor, taint_accum)
        }
        other => eval_expr_operator(other, stack, store),
    }
}

// ===== Public API =====

pub fn eval_expr_with_store(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    expr: ExprIdx,
) -> Result<(SlotValue, Taint), EngineError> {
    eval_expr_inner(plan, run, store, expr)
}

pub fn eval_expr(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    expr: ExprIdx,
) -> Result<(SlotValue, Taint), EngineError> {
    let mut store = ValueStore::new();
    eval_expr_inner(plan, run, &mut store, expr)
}
