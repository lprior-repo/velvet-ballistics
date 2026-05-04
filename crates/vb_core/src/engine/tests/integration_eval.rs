//! Integration tests for expression evaluation.

use crate::errors::EngineError;
use crate::frame::StepState;
use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram,
    PathSegment, ResourceContract, WorkflowParts,
};

use crate::engine::{EngineSignal, StepBudget, eval_accessor, eval_accessor_with_store, eval_expr,
    new_run_frame, run_until_blocked, step_once};

fn test_store() -> crate::value_store::ValueStore {
    crate::value_store::ValueStore::new()
}

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

fn test_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<crate::RunFrame, String> {
    new_run_frame(run_id, workflow).map_err(|error| error.to_string())
}

#[test]
fn public_eval_expr_returns_exact_value() -> Result<(), String> {
    let workflow = eval_add_workflow().map_err(|error| error.to_string())?;
    let run = test_frame(RunId::new(23), &workflow)?;

    let (value, _taint) =
        eval_expr(&workflow, &run, ExprIdx::new(0)).map_err(|error| error.to_string())?;

    ensure_equal(value, SlotValue::I64(42))?;
    Ok(())
}

#[test]
fn public_eval_expr_rejects_invalid_expr_index() -> Result<(), String> {
    let workflow = eval_add_workflow().map_err(|error| error.to_string())?;
    let run = test_frame(RunId::new(26), &workflow)?;

    match eval_expr(&workflow, &run, ExprIdx::new(1)) {
        Err(EngineError::ExprOutOfBounds { expr }) if expr == ExprIdx::new(1) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_expr_node_uses_fixed_stack_and_writes_output() -> Result<(), String> {
    let workflow = eval_add_workflow().map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(14), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    if result == EngineSignal::Finished(SlotValue::I64(42), Taint::Clean) {
        Ok(())
    } else {
        Err(format!("unexpected result: {result:?}"))
    }
}

#[test]
fn eval_expr_division_by_zero_returns_division_by_zero_error() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ]
        .into_boxed_slice(),
    )
    .map_err(|error| error.to_string())?;
    let parts = WorkflowParts {
        name: Box::<str>::from("div_zero"),
        digest: WorkflowDigest::from_bytes([0x11; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: vec![expression].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(10), ConstValue::I64(0)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow =
        CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(113), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store);

    match result {
        Err(EngineError::DivisionByZero) => {
            ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_expr_integer_overflow_returns_invalid_compiled_workflow() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Mul,
        ]
        .into_boxed_slice(),
    )
    .map_err(|error| error.to_string())?;
    let parts = WorkflowParts {
        name: Box::<str>::from("int_overflow"),
        digest: WorkflowDigest::from_bytes([0x22; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: vec![expression].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(i64::MAX)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow =
        CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(114), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store);

    match result {
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "integer arithmetic overflow",
        }) => {
            ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_expr_not_on_non_bool_returns_type_mismatch() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(
        vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not].into_boxed_slice(),
    )
    .map_err(|error| error.to_string())?;
    let parts = WorkflowParts {
        name: Box::<str>::from("not_on_int"),
        digest: WorkflowDigest::from_bytes([0x33; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: vec![expression].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow =
        CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(115), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store);

    match result {
        Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: "number",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

fn eval_add_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
    let expression = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ]
        .into_boxed_slice(),
    )
    .map_err(crate::WorkflowError::Expression)?;
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("eval_add"),
        digest: WorkflowDigest::from_bytes([7; 32]),
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
        expressions: vec![expression].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(19), ConstValue::I64(23)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
}
