#![forbid(unsafe_code)]
//! Integration tests for choose slot and choose expression nodes.

use crate::ids::{ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp, ExprProgram, SlotBranch,
    WorkflowParts,
};

use crate::engine::{EngineSignal, StepBudget, new_run_frame, run_until_blocked};

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

// ===== ChooseSlot tests =====

#[test]
fn choose_slot_takes_first_true_branch() -> Result<(), String> {
    let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(8), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(true), Taint::Clean)
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(true), Taint::Clean)
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    if result == EngineSignal::Finished(SlotValue::I64(11), Taint::Clean) {
        Ok(())
    } else {
        Err(format!("unexpected result: {result:?}"))
    }
}

#[test]
fn choose_slot_takes_later_true_branch() -> Result<(), String> {
    let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(10), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(false), Taint::Clean)
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(true), Taint::Clean)
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    if result == EngineSignal::Finished(SlotValue::I64(22), Taint::Clean) {
        Ok(())
    } else {
        Err(format!("unexpected result: {result:?}"))
    }
}

#[test]
fn choose_slot_takes_otherwise_when_no_branch_matches() -> Result<(), String> {
    let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(9), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(false), Taint::Clean)
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(false), Taint::Clean)
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    if result == EngineSignal::Finished(SlotValue::I64(99), Taint::Clean) {
        Ok(())
    } else {
        Err(format!("unexpected result: {result:?}"))
    }
}

#[test]
fn choose_slot_rejects_non_bool_condition_with_type_mismatch() -> Result<(), String> {
    let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(11), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    match run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store) {
        Err(crate::errors::EngineError::TypeMismatch {
            expected: "boolean",
            found: "number",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn choose_slot_otherwise_taken_when_no_branch_matches() -> Result<(), String> {
    let workflow = choose_slot_without_otherwise_workflow().map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(12), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(false), Taint::Clean)
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(false), Taint::Clean)
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    if result == EngineSignal::Finished(SlotValue::I64(99), Taint::Clean) {
        Ok(())
    } else {
        Err(format!("unexpected result: {result:?}"))
    }
}

// ===== ChooseExpr tests =====

#[test]
fn choose_expr_takes_first_true_branch() -> Result<(), String> {
    let workflow = choose_expr_workflow().map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(13), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    if result == EngineSignal::Finished(SlotValue::I64(11), Taint::Clean) {
        Ok(())
    } else {
        Err(format!("unexpected result: {result:?}"))
    }
}

#[test]
fn choose_expr_takes_later_true_branch() -> Result<(), String> {
    let workflow = choose_expr_workflow_with(
        ConstValue::Bool(false),
        ConstValue::Bool(true),
        Some(StepIdx::new(3)),
    )
    .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(20), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(22), Taint::Clean),
    )?;
    Ok(())
}

#[test]
fn choose_expr_takes_otherwise_when_all_false() -> Result<(), String> {
    let workflow = choose_expr_workflow_with(
        ConstValue::Bool(false),
        ConstValue::Bool(false),
        Some(StepIdx::new(3)),
    )
    .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(21), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(99), Taint::Clean),
    )?;
    Ok(())
}

#[test]
fn choose_expr_rejects_non_bool_condition() -> Result<(), String> {
    let workflow = choose_expr_workflow_with(
        ConstValue::I64(1),
        ConstValue::Bool(true),
        Some(StepIdx::new(3)),
    )
    .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(22), &workflow)?;
    let mut store = test_store();

    match run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store) {
        Err(crate::errors::EngineError::TypeMismatch {
            expected: "boolean",
            found: "number",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn choose_expr_otherwise_taken_when_no_branch_matches() -> Result<(), String> {
    let workflow = choose_expr_workflow_with(
        ConstValue::Bool(false),
        ConstValue::Bool(false),
        Some(StepIdx::new(3)),
    )
    .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(25), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    if result == EngineSignal::Finished(SlotValue::I64(99), Taint::Clean) {
        Ok(())
    } else {
        Err(format!("unexpected result: {result:?}"))
    }
}

// ===== Workflow helpers =====

fn choose_slot_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
    choose_slot_workflow_with_otherwise(Some(StepIdx::new(3)))
}

fn choose_slot_without_otherwise_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
    choose_slot_workflow_with_otherwise(Some(StepIdx::new(3)))
}

fn choose_slot_workflow_with_otherwise(
    otherwise: Option<StepIdx>,
) -> Result<CompiledWorkflow, crate::WorkflowError> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: vec![
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    SlotBranch {
                        condition: SlotIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]
                .into_boxed_slice(),
                otherwise,
            },
        },
        set_const_node(1, 2, 0),
        set_const_node(2, 2, 1),
        set_const_node(3, 2, 2),
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        },
    ];
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("choose_slot"),
        digest: WorkflowDigest::from_bytes([5; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![
            ConstValue::I64(11),
            ConstValue::I64(22),
            ConstValue::I64(99),
        ]
        .into_boxed_slice(),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
}

fn choose_expr_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
    choose_expr_workflow_with(
        ConstValue::Bool(true),
        ConstValue::Bool(false),
        Some(StepIdx::new(3)),
    )
}

fn choose_expr_workflow_with(
    first: ConstValue,
    second: ConstValue,
    otherwise: Option<StepIdx>,
) -> Result<CompiledWorkflow, crate::WorkflowError> {
    let true_expr =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())
            .map_err(crate::WorkflowError::Expression)?;
    let false_expr =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into_boxed_slice())
            .map_err(crate::WorkflowError::Expression)?;
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: vec![
                    ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    ExprBranch {
                        condition: ExprIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]
                .into_boxed_slice(),
                otherwise,
            },
        },
        set_const_node(1, 2, 2),
        set_const_node(2, 2, 3),
        set_const_node(3, 2, 4),
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        },
    ];
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("choose_expr"),
        digest: WorkflowDigest::from_bytes([6; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: vec![true_expr, false_expr].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![
            first,
            second,
            ConstValue::I64(11),
            ConstValue::I64(22),
            ConstValue::I64(99),
        ]
        .into_boxed_slice(),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
}

fn set_const_node(id: u16, output: u16, constant: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: Some(SlotIdx::new(output)),
        next: Some(StepIdx::new(4)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(constant),
        },
    }
}
