#![forbid(unsafe_code)]
//! Tests for expression evaluation core.

use crate::errors::EngineError;
use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram,
    ResourceContract, WorkflowParts,
};

use super::{eval_expr, eval_expr_with_store};

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

    let max_stack =
        check_expr_stack_bound(&expr_ops, MAX_EXPRESSION_STACK).map_err(|e| e.to_string())?;
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
    let (value, taint) = eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
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
    let workflow = make_workflow(vec![ExprOp::LoadSlot(SlotIdx::new(0))], vec![], 2)?;
    let mut run = make_run(2)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
        .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();
    let (value, _taint) = eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Bool(true))
}

#[test]
fn eval_load_slot_propagates_secret_taint() -> Result<(), String> {
    let workflow = make_workflow(vec![ExprOp::LoadSlot(SlotIdx::new(0))], vec![], 2)?;
    let mut run = make_run(2)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret)
        .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();
    let (_value, taint) = eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
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
    run.write_slot_with_taint(
        SlotIdx::new(1),
        SlotValue::I64(20),
        Taint::DerivedFromSecret,
    )
    .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();
    let (value, taint) = eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
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
    let max_stack =
        check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK).map_err(|e| e.to_string())?;
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

    let (value, _taint) = eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
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
    let (value, _taint) = eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(90))
}

// ===== eval_load_slot error branches =====

#[test]
fn eval_load_slot_rejects_out_of_bounds_slot() -> Result<(), String> {
    let workflow = make_workflow(vec![ExprOp::LoadSlot(SlotIdx::new(5))], vec![], 1)?;
    let run = make_run(1)?;
    let mut store = ValueStore::new();
    let result = eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0));
    match result {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
        other => Err(format!("expected SlotOutOfBounds, got {other:?}")),
    }
}

#[test]
fn eval_load_slot_rejects_uninitialized_slot() -> Result<(), String> {
    let workflow = make_workflow(vec![ExprOp::LoadSlot(SlotIdx::new(0))], vec![], 2)?;
    let run = make_run(2)?; // slot 0 is uninitialized
    let mut store = ValueStore::new();
    let result = eval_expr_with_store(&workflow, &run, &mut store, ExprIdx::new(0));
    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(0) => Ok(()),
        other => Err(format!("expected SlotUninitialized, got {other:?}")),
    }
}
