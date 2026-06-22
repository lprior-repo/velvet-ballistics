#![forbid(unsafe_code)]
//! Integration tests for workflow execution, errors, and taint propagation.

use crate::errors::EngineError;
use crate::frame::StepState;
use crate::ids::{
    ActionId, ConstIdx, ExprIdx, RunId, SeqNo, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::value::{ConstValue, SlotValue, Taint, join_taint};
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, WorkflowParts,
};

use crate::engine::{EngineSignal, StepBudget, new_run_frame, run_until_blocked, step_once};

fn test_store() -> ValueStore {
    ValueStore::new()
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

// ===== Workflow error handling =====

#[test]
fn step_once_with_invalid_pc_rejected_by_set_pc() -> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(103), &workflow)?;

    let result = run.set_pc(StepIdx::new(99));

    match result {
        Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn nop_without_next_returns_missing_next_step() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("nop_no_next"),
        digest: WorkflowDigest::from_bytes([0xAA; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(104), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store);

    match result {
        Err(EngineError::MissingNextStep { step }) if step == StepIdx::new(0) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn set_const_without_output_slot_returns_missing_output_slot() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("set_const_no_output"),
        digest: WorkflowDigest::from_bytes([0xBB; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
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
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(105), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store);

    match result {
        Err(EngineError::MissingOutputSlot { step }) if step == StepIdx::new(0) => {
            ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn finish_with_uninitialized_result_slot_returns_slot_uninitialized() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("finish_empty_slot"),
        digest: WorkflowDigest::from_bytes([0xCC; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(106), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store);

    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(0) => {
            ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn failed_step_is_marked_failed_in_frame_after_engine_error() -> Result<(), String> {
    let workflow = copy_workflow(None).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(107), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store);

    ensure_equal(
        result,
        Err(EngineError::MissingOutputSlot {
            step: StepIdx::new(0),
        }),
    )?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
    Ok(())
}

#[test]
fn set_pc_to_out_of_bounds_target_returns_invalid_program_counter() -> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(108), &workflow)?;

    let result = run.set_pc(StepIdx::new(200));

    match result {
        Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(200) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn copy_from_uninitialized_source_slot_returns_slot_uninitialized() -> Result<(), String> {
    let workflow = copy_workflow(Some(SlotIdx::new(1))).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(109), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store);

    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(0) => {
            ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ===== Awaiting signals =====

#[test]
fn drive_deterministic_stops_on_awaiting_action_signal() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("do_node"),
        digest: WorkflowDigest::from_bytes([0xDD; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(110), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

    ensure_equal(
        result,
        Ok(EngineSignal::AwaitingAction {
            step: StepIdx::new(0),
            seq: SeqNo::ZERO,
            action: ActionId::new(1),
        }),
    )?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))?;
    Ok(())
}

#[test]
fn drive_deterministic_stops_on_awaiting_wait_signal() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("wait_node"),
        digest: WorkflowDigest::from_bytes([0xEE; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(111), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

    ensure_equal(
        result,
        Ok(EngineSignal::AwaitingWait {
            deadline_slot: SlotIdx::new(0),
        }),
    )?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))?;
    Ok(())
}

#[test]
fn drive_deterministic_stops_on_awaiting_ask_signal() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("ask_node"),
        digest: WorkflowDigest::from_bytes([0xFF; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(112), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

    ensure_equal(result, Ok(EngineSignal::AwaitingAsk { timeout_slot: None }))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))?;
    Ok(())
}

// ===== Taint propagation =====

#[test]
fn eval_expr_with_secret_tainted_slot_produces_derived_from_secret_taint() -> Result<(), String> {
    let expression =
        ExprProgram::try_from_ops(vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice())
            .map_err(|error| error.to_string())?;
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("taint_eval_expr"),
        digest: WorkflowDigest::from_bytes([0x43; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
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
                    result: SlotIdx::new(1),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: vec![expression].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(200), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret)
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

    match result {
        Ok(EngineSignal::Finished(SlotValue::I64(99), Taint::Secret)) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_expr_with_clean_slot_produces_clean_taint() -> Result<(), String> {
    let expression =
        ExprProgram::try_from_ops(vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice())
            .map_err(|error| error.to_string())?;
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("taint_eval_clean"),
        digest: WorkflowDigest::from_bytes([0x43; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
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
                    result: SlotIdx::new(1),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: vec![expression].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(201), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(10))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

    match result {
        Ok(EngineSignal::Finished(SlotValue::I64(10), Taint::Clean)) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn build_object_joins_taint_from_all_field_slots() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("taint_build_object"),
        digest: WorkflowDigest::from_bytes([0x43; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![
                        (SymbolId::new(1), SlotIdx::new(0)),
                        (SymbolId::new(2), SlotIdx::new(1)),
                    ]
                    .into_boxed_slice(),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(10), ConstValue::I64(20)].into_boxed_slice(),
        slot_count: 3,
        symbols_count: 3,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(202), &workflow)?;
    let mut store = test_store();

    let s0 = step_once(&workflow, &mut run, &mut store);
    match s0 {
        Ok(EngineSignal::Continue) => {}
        other => return Err(format!("expected Continue from step 0, got {other:?}")),
    }
    let s1 = step_once(&workflow, &mut run, &mut store);
    match s1 {
        Ok(EngineSignal::Continue) => {}
        other => return Err(format!("expected Continue from step 1, got {other:?}")),
    }
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret)
        .map_err(|error| error.to_string())?;
    let s2 = step_once(&workflow, &mut run, &mut store);
    match s2 {
        Ok(EngineSignal::Continue) => {}
        other => return Err(format!("expected Continue from step 2, got {other:?}")),
    }
    let slot2_taint = run
        .read_taint(SlotIdx::new(2))
        .map_err(|error| error.to_string())?;
    ensure_equal(slot2_taint, Taint::Secret)?;
    let s3 = step_once(&workflow, &mut run, &mut store);
    match s3 {
        Ok(EngineSignal::Finished(SlotValue::Object(_), Taint::Secret)) => Ok(()),
        Ok(EngineSignal::Finished(_, other_taint)) => {
            Err(format!("expected Secret taint, got {other_taint:?}"))
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn build_object_with_all_clean_slots_produces_clean_taint() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("taint_build_object_clean"),
        digest: WorkflowDigest::from_bytes([0x43; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![
                        (SymbolId::new(1), SlotIdx::new(0)),
                        (SymbolId::new(2), SlotIdx::new(1)),
                    ]
                    .into_boxed_slice(),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(10), ConstValue::I64(20)].into_boxed_slice(),
        slot_count: 3,
        symbols_count: 3,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(203), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

    match result {
        Ok(EngineSignal::Finished(SlotValue::Object(_), Taint::Clean)) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn build_list_joins_taint_from_all_item_slots() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("taint_build_list"),
        digest: WorkflowDigest::from_bytes([0x43; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0), SlotIdx::new(1)].into_boxed_slice(),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(11), ConstValue::I64(22)].into_boxed_slice(),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(204), &workflow)?;
    let mut store = test_store();

    let s0 = step_once(&workflow, &mut run, &mut store);
    match s0 {
        Ok(EngineSignal::Continue) => {}
        other => return Err(format!("expected Continue from step 0, got {other:?}")),
    }
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::I64(11),
        Taint::DerivedFromSecret,
    )
    .map_err(|error| error.to_string())?;
    let s1 = step_once(&workflow, &mut run, &mut store);
    match s1 {
        Ok(EngineSignal::Continue) => {}
        other => return Err(format!("expected Continue from step 1, got {other:?}")),
    }
    let s2 = step_once(&workflow, &mut run, &mut store);
    match s2 {
        Ok(EngineSignal::Continue) => {}
        other => return Err(format!("expected Continue from step 2, got {other:?}")),
    }
    let slot2_taint = run
        .read_taint(SlotIdx::new(2))
        .map_err(|error| error.to_string())?;
    ensure_equal(slot2_taint, Taint::DerivedFromSecret)?;
    let s3 = step_once(&workflow, &mut run, &mut store);
    match s3 {
        Ok(EngineSignal::Finished(SlotValue::List(_), Taint::DerivedFromSecret)) => Ok(()),
        Ok(EngineSignal::Finished(_, other_taint)) => Err(format!(
            "expected DerivedFromSecret taint, got {other_taint:?}"
        )),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn build_list_with_all_secret_slots_produces_secret_taint() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("taint_build_list_all_secret"),
        digest: WorkflowDigest::from_bytes([0x43; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0), SlotIdx::new(1)].into_boxed_slice(),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(11), ConstValue::I64(22)].into_boxed_slice(),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(205), &workflow)?;
    let mut store = test_store();

    let s0 = step_once(&workflow, &mut run, &mut store);
    match s0 {
        Ok(EngineSignal::Continue) => {}
        other => return Err(format!("expected Continue from step 0, got {other:?}")),
    }
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(11), Taint::Secret)
        .map_err(|error| error.to_string())?;
    let s1 = step_once(&workflow, &mut run, &mut store);
    match s1 {
        Ok(EngineSignal::Continue) => {}
        other => return Err(format!("expected Continue from step 1, got {other:?}")),
    }
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(22), Taint::Secret)
        .map_err(|error| error.to_string())?;
    let s2 = step_once(&workflow, &mut run, &mut store);
    match s2 {
        Ok(EngineSignal::Continue) => {}
        other => return Err(format!("expected Continue from step 2, got {other:?}")),
    }
    let slot2_taint = run
        .read_taint(SlotIdx::new(2))
        .map_err(|error| error.to_string())?;
    ensure_equal(slot2_taint, Taint::Secret)?;
    let s3 = step_once(&workflow, &mut run, &mut store);
    match s3 {
        Ok(EngineSignal::Finished(SlotValue::List(_), Taint::Secret)) => Ok(()),
        Ok(EngineSignal::Finished(_, other_taint)) => {
            Err(format!("expected Secret taint, got {other_taint:?}"))
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn engine_signal_finished_carries_correct_secret_taint() -> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::I64(77)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(206), &workflow)?;
    let mut store = test_store();

    let first = step_once(&workflow, &mut run, &mut store);
    match first {
        Ok(EngineSignal::Continue) => {}
        other => return Err(format!("expected Continue from first step, got {other:?}")),
    }
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Secret)
        .map_err(|error| error.to_string())?;

    let second = step_once(&workflow, &mut run, &mut store);
    match second {
        Ok(EngineSignal::Finished(SlotValue::I64(77), Taint::Secret)) => Ok(()),
        Ok(EngineSignal::Finished(value, taint)) => Err(format!(
            "expected Finished(I64(77), Secret), got ({value:?}, {taint:?})"
        )),
        other => Err(format!("expected Finished, got {other:?}")),
    }
}

#[test]
fn engine_signal_finished_carries_correct_derived_from_secret_taint() -> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::Bool(true)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(207), &workflow)?;
    let mut store = test_store();

    let first = step_once(&workflow, &mut run, &mut store);
    match first {
        Ok(EngineSignal::Continue) => {}
        other => return Err(format!("expected Continue from first step, got {other:?}")),
    }
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )
    .map_err(|error| error.to_string())?;

    let second = step_once(&workflow, &mut run, &mut store);
    match second {
        Ok(EngineSignal::Finished(SlotValue::Bool(true), Taint::DerivedFromSecret)) => Ok(()),
        Ok(EngineSignal::Finished(value, taint)) => Err(format!(
            "expected Finished(Bool(true), DerivedFromSecret), got ({value:?}, {taint:?})"
        )),
        other => Err(format!("expected Finished, got {other:?}")),
    }
}

// ===== join_taint tests =====

#[test]
fn join_taint_clean_plus_clean_is_clean() {
    assert_eq!(join_taint(Taint::Clean, Taint::Clean), Taint::Clean);
}

#[test]
fn join_taint_clean_plus_secret_is_secret() {
    assert_eq!(join_taint(Taint::Clean, Taint::Secret), Taint::Secret);
}

#[test]
fn join_taint_secret_plus_clean_is_secret() {
    assert_eq!(join_taint(Taint::Secret, Taint::Clean), Taint::Secret);
}

#[test]
fn join_taint_clean_plus_derived_from_secret_is_derived_from_secret() {
    assert_eq!(
        join_taint(Taint::Clean, Taint::DerivedFromSecret),
        Taint::DerivedFromSecret
    );
}

#[test]
fn join_taint_derived_from_secret_plus_clean_is_derived_from_secret() {
    assert_eq!(
        join_taint(Taint::DerivedFromSecret, Taint::Clean),
        Taint::DerivedFromSecret
    );
}

#[test]
fn join_taint_secret_plus_derived_from_secret_is_secret() {
    assert_eq!(
        join_taint(Taint::Secret, Taint::DerivedFromSecret),
        Taint::Secret
    );
}

#[test]
fn join_taint_derived_from_secret_plus_secret_is_secret() {
    assert_eq!(
        join_taint(Taint::DerivedFromSecret, Taint::Secret),
        Taint::Secret
    );
}

// ===== Workflow helpers =====

fn tiny_workflow(value: ConstValue) -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("tiny"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
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
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![value].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
}

fn copy_workflow(output: Option<SlotIdx>) -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("copy"),
        digest: WorkflowDigest::from_bytes([9; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
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
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
}
