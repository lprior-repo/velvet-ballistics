// Numeric operator execution and symbolic property coverage
// Uses validated EvalExpr->Finish drive through real workflow.

use crate::errors::EngineError;
use crate::ids::{ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::value::{ConstValue, FiniteF64, SlotValue, Taint};
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract, WorkflowParts,
    check_expr_stack_bound, node,
};

fn f64_const(value: f64) -> Result<ConstValue, String> {
    FiniteF64::new(value)
        .map(ConstValue::F64)
        .map_err(|e| e.to_string())
}

fn drive_numeric_expr(
    ops: Vec<ExprOp>,
    constants: Vec<ConstValue>,
    slots: Vec<(SlotValue, Taint)>,
) -> Result<crate::EngineSignal, EngineError> {
    let expr = numeric_expr_program(ops)?;
    let slot_count = numeric_slot_count(&slots)?;
    let workflow = numeric_workflow(expr, constants, slot_count)?;
    let mut run = numeric_run(slot_count, &slots)?;
    let mut store = crate::value_store::ValueStore::new();
    drive_numeric_workflow(&workflow, &mut run, &mut store)
}

type NumericScenario = (&'static str, Vec<ExprOp>, Vec<ConstValue>, SlotValue);

include!("numeric_workflow_fixture_tests.rs");
include!("numeric_arithmetic_tests.rs");
include!("numeric_comparison_execution_tests.rs");

fn check_numeric_scenario((name, ops, constants, expected): NumericScenario) -> Result<(), String> {
    let signal = drive_numeric_expr(ops, constants, vec![]).map_err(|e| format!("{name}: {e}"))?;
    match signal {
        crate::EngineSignal::Finished(value, Taint::Clean) => {
            if value != expected {
                return Err(format!("{name}: expected {expected:?}, got {value:?}"));
            }
        }
        other => {
            return Err(format!("{name}: unexpected signal {other:?}"));
        }
    }
    Ok(())
}

#[test]
fn numeric_ops_all_pairings_through_drive() -> Result<(), String> {
    let scenarios = ARITHMETIC_SCENARIOS
        .into_iter()
        .chain(COMPARISON_SCENARIOS)
        .map(|scenario| scenario())
        .collect::<Result<Vec<_>, String>>()?;
    for scenario in scenarios {
        check_numeric_scenario(scenario)?;
    }
    Ok(())
}

#[test]
fn taint_joins_to_secret_through_arithmetic_drive() -> Result<(), String> {
    let signal = drive_numeric_expr(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Add,
        ],
        vec![],
        vec![
            (SlotValue::I64(6), Taint::Clean),
            (SlotValue::I64(2), Taint::Secret),
        ],
    )
    .map_err(|e| e.to_string())?;
    match signal {
        crate::EngineSignal::Finished(SlotValue::I64(8), Taint::Secret) => Ok(()),
        other => Err(format!("unexpected: {other:?}")),
    }
}

#[test]
fn taint_joins_to_secret_through_comparison_drive() -> Result<(), String> {
    let signal = drive_numeric_expr(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Gt,
        ],
        vec![],
        vec![
            (SlotValue::I64(6), Taint::Clean),
            (SlotValue::I64(2), Taint::Secret),
        ],
    )
    .map_err(|e| e.to_string())?;
    match signal {
        crate::EngineSignal::Finished(SlotValue::Bool(true), Taint::Secret) => Ok(()),
        other => Err(format!("unexpected: {other:?}")),
    }
}

#[test]
fn i64_min_by_neg_one_division_overflow_through_drive() -> Result<(), String> {
    let signal = drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![ConstValue::I64(i64::MIN), ConstValue::I64(-1)],
        vec![],
    );
    match signal {
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "integer division overflow",
        }) => Ok(()),
        other => Err(format!("expected division overflow, got {other:?}")),
    }
}

#[test]
fn div_by_zero_i64_through_drive() -> Result<(), String> {
    let signal = drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![ConstValue::I64(10), ConstValue::I64(0)],
        vec![],
    );
    match signal {
        Err(EngineError::DivisionByZero) => Ok(()),
        other => Err(format!("expected DivisionByZero, got {other:?}")),
    }
}

#[test]
fn f64_zero_division_returns_non_finite_error_through_drive() -> Result<(), String> {
    let signal = drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![f64_const(1.0)?, f64_const(0.0)?],
        vec![],
    );
    match signal {
        Err(EngineError::NonFiniteNumber) => Ok(()),
        other => Err(format!("expected NonFiniteNumber, got {other:?}")),
    }
}

include!("numeric_property_tests.rs");

#[cfg(test)]
#[path = "numeric_boundary_tests.rs"]
mod numeric_boundary;
