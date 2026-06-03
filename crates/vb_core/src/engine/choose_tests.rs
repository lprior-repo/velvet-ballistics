#![forbid(unsafe_code)]
//! Tests for choose branch evaluation logic.

use crate::errors::EngineError;
use crate::ids::{ExprIdx, RunId, SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::workflow::{ExprBranch, SlotBranch};
use crate::engine::choose::{choose_expr_branch, choose_expr_target, choose_slot_branch, choose_slot_target};
use crate::frame::RunFrame;
use crate::ValueStore;

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

fn test_frame(slot_count: u16) -> Result<RunFrame, String> {
    RunFrame::new(RunId::new(1), StepIdx::new(0), 10, slot_count).map_err(|e| e.to_string())
}

// ===== choose_slot_branch tests =====

#[test]
fn choose_slot_takes_first_matching_branch() -> Result<(), String> {
    let mut run = test_frame(3)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))
        .map_err(|e| e.to_string())?;

    let branches = vec![
        SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(5),
        },
        SlotBranch {
            condition: SlotIdx::new(1),
            target: StepIdx::new(7),
        },
    ];
    let result =
        choose_slot_branch(&mut run, &branches, Some(StepIdx::new(9))).map_err(|e| e.to_string())?;

    ensure_equal(result, crate::EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(5))
}

#[test]
fn choose_slot_skips_false_takes_second_branch() -> Result<(), String> {
    let mut run = test_frame(3)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))
        .map_err(|e| e.to_string())?;

    let branches = vec![
        SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(5),
        },
        SlotBranch {
            condition: SlotIdx::new(1),
            target: StepIdx::new(7),
        },
    ];
    let result =
        choose_slot_branch(&mut run, &branches, Some(StepIdx::new(9))).map_err(|e| e.to_string())?;

    ensure_equal(result, crate::EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(7))
}

#[test]
fn choose_slot_takes_otherwise_when_all_false() -> Result<(), String> {
    let mut run = test_frame(2)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(false))
        .map_err(|e| e.to_string())?;

    let branches = vec![
        SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(5),
        },
        SlotBranch {
            condition: SlotIdx::new(1),
            target: StepIdx::new(7),
        },
    ];
    let _result =
        choose_slot_branch(&mut run, &branches, Some(StepIdx::new(3))).map_err(|e| e.to_string())?;

    ensure_equal(run.pc(), StepIdx::new(3))
}

#[test]
fn choose_slot_without_otherwise_returns_error() -> Result<(), String> {
    let mut run = test_frame(1)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
        .map_err(|e| e.to_string())?;

    let branches = vec![SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(5),
    }];
    let result = choose_slot_branch(&mut run, &branches, None);

    match result {
        Err(EngineError::MissingNextStep { step: _ }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn choose_slot_rejects_non_bool_condition() -> Result<(), String> {
    let mut run = test_frame(1)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|e| e.to_string())?;

    let branches = vec![SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(5),
    }];
    let result = choose_slot_branch(&mut run, &branches, Some(StepIdx::new(3)));

    match result {
        Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: "number",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn choose_slot_empty_branches_takes_otherwise() -> Result<(), String> {
    let mut run = test_frame(1)?;
    let branches: Vec<SlotBranch> = vec![];
    let _result =
        choose_slot_branch(&mut run, &branches, Some(StepIdx::new(2))).map_err(|e| e.to_string())?;

    ensure_equal(run.pc(), StepIdx::new(2))
}

// ===== choose_expr_branch tests =====

fn minimal_expr_plan() -> Result<crate::workflow::CompiledWorkflow, String> {
    use crate::ids::{ConstIdx, ExprIdx, WorkflowDigest};
    use crate::value::ConstValue;
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, WorkflowParts,
    };

    let expr_true =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())
            .map_err(crate::WorkflowError::Expression)
            .map_err(|e| e.to_string())?;
    let expr_false =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into_boxed_slice())
            .map_err(crate::WorkflowError::Expression)
            .map_err(|e| e.to_string())?;

    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("choose_expr_test"),
        digest: WorkflowDigest::from_bytes([0x55; 32]),
        nodes: vec![
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
                    otherwise: Some(StepIdx::new(3)),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
        ]
        .into_boxed_slice(),
        expressions: vec![expr_true, expr_false].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::Bool(true), ConstValue::Bool(false)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

#[test]
fn choose_expr_takes_first_true_branch() -> Result<(), String> {
    let plan = minimal_expr_plan()?;
    let mut run = test_frame(1)?;
    let mut store = ValueStore::new();

    let branches = vec![
        ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(1),
        },
        ExprBranch {
            condition: ExprIdx::new(1),
            target: StepIdx::new(2),
        },
    ];
    let result = choose_expr_branch(&plan, &mut run, &mut store, &branches, Some(StepIdx::new(3)))
        .map_err(|e| e.to_string())?;

    ensure_equal(result, crate::EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))
}

#[test]
fn choose_expr_takes_otherwise_when_all_false() -> Result<(), String> {
    let plan = minimal_expr_plan()?;
    let mut run = test_frame(1)?;
    let mut store = ValueStore::new();

    let branches = vec![ExprBranch {
        condition: ExprIdx::new(1),
        target: StepIdx::new(2),
    }];
    let _result =
        choose_expr_branch(&plan, &mut run, &mut store, &branches, Some(StepIdx::new(3)))
            .map_err(|e| e.to_string())?;

    ensure_equal(run.pc(), StepIdx::new(3))
}
