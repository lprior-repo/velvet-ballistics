//! Expression evaluation core.

use crate::errors::EngineError;
use crate::ids::{ConstIdx, ExprIdx, SlotIdx};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{CompiledWorkflow, ExprOp};

use super::accessors::eval_load_accessor;
use super::ops::eval_expr_operator;
use super::stack::{push_value, ExprStack};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{
        AccessorIdx, ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
    };
    use crate::value::{ConstValue, SlotValue, Taint};
    use crate::workflow::{
        AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram,
        PathSegment, ResourceContract, WorkflowParts,
    };
    use crate::value_store::ValueStore;

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

    fn make_workflow(
        expr_ops: Vec<ExprOp>,
        constants: Vec<ConstValue>,
        slot_count: u16,
    ) -> Result<CompiledWorkflow, String> {
        use crate::limits::MAX_EXPRESSION_STACK;
        use crate::workflow::check_expr_stack_bound;

        let max_stack = check_expr_stack_bound(&expr_ops, MAX_EXPRESSION_STACK)
            .map_err(|e| e.to_string())?;
        let expr = ExprProgram::try_from_parts(expr_ops.into_boxed_slice(), max_stack)
            .map_err(|e| e.to_string())?;
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("core_test"),
            digest: WorkflowDigest::from_bytes([0xFC; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: constants.into_boxed_slice(),
            slot_count,
            symbols_count: 10,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())
    }

    fn make_run(slot_count: u16) -> Result<crate::frame::RunFrame, String> {
        crate::frame::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, slot_count)
            .map_err(|e| e.to_string())
    }

    // ===== LoadConst =====

    #[test]
    fn eval_load_const_returns_constant_value() -> Result<(), String> {
        let workflow = make_workflow(
            vec![ExprOp::LoadConst(ConstIdx::new(0))],
            vec![ConstValue::I64(42)],
            1,
        )?;
        let run = make_run(1)?;
        let mut store = ValueStore::new();
        let (value, taint) =
            eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
                .map_err(|e| e.to_string())?;
        ensure_equal(value, SlotValue::I64(42))?;
        ensure_equal(taint, Taint::Clean)
    }

    #[test]
    fn eval_load_const_rejects_out_of_bounds() -> Result<(), String> {
        let workflow = make_workflow(
            vec![ExprOp::LoadConst(ConstIdx::new(5))],
            vec![ConstValue::I64(1)],
            1,
        )?;
        let run = make_run(1)?;
        let mut store = ValueStore::new();
        let result = eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0));
        match result {
            Err(EngineError::ConstOutOfBounds { index }) if index == ConstIdx::new(5) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // ===== LoadSlot =====

    #[test]
    fn eval_load_slot_reads_slot_value() -> Result<(), String> {
        let workflow = make_workflow(
            vec![ExprOp::LoadSlot(SlotIdx::new(0))],
            vec![],
            2,
        )?;
        let mut run = make_run(2)?;
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
            .map_err(|e| e.to_string())?;
        let mut store = ValueStore::new();
        let (value, _taint) =
            eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
                .map_err(|e| e.to_string())?;
        ensure_equal(value, SlotValue::Bool(true))
    }

    #[test]
    fn eval_load_slot_propagates_secret_taint() -> Result<(), String> {
        let workflow = make_workflow(
            vec![ExprOp::LoadSlot(SlotIdx::new(0))],
            vec![],
            2,
        )?;
        let mut run = make_run(2)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret)
            .map_err(|e| e.to_string())?;
        let mut store = ValueStore::new();
        let (_value, taint) =
            eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
                .map_err(|e| e.to_string())?;
        ensure_equal(taint, Taint::Secret)
    }

    #[test]
    fn eval_load_slot_joins_taint_from_multiple_slots() -> Result<(), String> {
        let workflow = make_workflow(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Add,
            ],
            vec![],
            2,
        )?;
        let mut run = make_run(2)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean)
            .map_err(|e| e.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::DerivedFromSecret)
            .map_err(|e| e.to_string())?;
        let mut store = ValueStore::new();
        let (value, taint) =
            eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
                .map_err(|e| e.to_string())?;
        ensure_equal(value, SlotValue::I64(30))?;
        ensure_equal(taint, Taint::DerivedFromSecret)
    }

    // ===== ExprOutOfBounds =====

    #[test]
    fn eval_expr_rejects_out_of_bounds_expr_index() -> Result<(), String> {
        let workflow = make_workflow(
            vec![ExprOp::LoadConst(ConstIdx::new(0))],
            vec![ConstValue::I64(1)],
            1,
        )?;
        let run = make_run(1)?;
        let mut store = ValueStore::new();
        let result = eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(1));
        match result {
            Err(EngineError::ExprOutOfBounds { expr }) if expr == ExprIdx::new(1) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // ===== Empty expression produces error =====

    #[test]
    fn eval_empty_expression_rejected_by_stack_bound_check() -> Result<(), String> {
        // Empty expression ops cause check_expr_stack_bound to fail at the
        // workflow construction stage, since the final stack depth is 0 (underflow).
        let result = make_workflow(vec![], vec![], 1);
        match result {
            Err(msg) if msg.contains("expression stack underflow") => Ok(()),
            Err(msg) if msg.contains("non-single") => Ok(()),
            other => Err(format!("expected stack error, got {other:?}")),
        }
    }

    // ===== LoadAccessor through expression =====

    #[test]
    fn eval_load_accessor_with_empty_path_reads_root() -> Result<(), String> {
        use crate::limits::MAX_EXPRESSION_STACK;
        use crate::workflow::check_expr_stack_bound;

        let ops = vec![ExprOp::LoadAccessor(AccessorIdx::new(0))];
        let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK)
            .map_err(|e| e.to_string())?;
        let expr = ExprProgram::try_from_parts(ops.into_boxed_slice(), max_stack)
            .map_err(|e| e.to_string())?;

        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("accessor_expr_test"),
            digest: WorkflowDigest::from_bytes([0xFD; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: vec![AccessorProgram {
                root: SlotIdx::new(0),
                path: Box::new([]),
            }]
            .into_boxed_slice(),
            constants: Box::new([]),
            slot_count: 2,
            symbols_count: 10,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;

        let mut run = make_run(2)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
            .map_err(|e| e.to_string())?;
        let mut store = ValueStore::new();

        let (value, _taint) =
            eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
                .map_err(|e| e.to_string())?;
        ensure_equal(value, SlotValue::I64(77))
    }

    // ===== eval_expr (without store) =====

    #[test]
    fn eval_expr_without_store_loads_const() -> Result<(), String> {
        let workflow = make_workflow(
            vec![ExprOp::LoadConst(ConstIdx::new(0))],
            vec![ConstValue::Bool(false)],
            1,
        )?;
        let run = make_run(1)?;
        let (value, taint) =
            eval_expr(&workflow, &run, ExprIdx::new(0)).map_err(|e| e.to_string())?;
        ensure_equal(value, SlotValue::Bool(false))?;
        ensure_equal(taint, Taint::Clean)
    }

    // ===== Multi-op expression =====

    #[test]
    fn eval_multi_op_expression_produces_correct_result() -> Result<(), String> {
        // (10 + 20) * 3 = 90
        let workflow = make_workflow(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
                ExprOp::LoadConst(ConstIdx::new(2)),
                ExprOp::Mul,
            ],
            vec![ConstValue::I64(10), ConstValue::I64(20), ConstValue::I64(3)],
            1,
        )?;
        let run = make_run(1)?;
        let mut store = ValueStore::new();
        let (value, _taint) =
            eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
                .map_err(|e| e.to_string())?;
        ensure_equal(value, SlotValue::I64(90))
    }
}
