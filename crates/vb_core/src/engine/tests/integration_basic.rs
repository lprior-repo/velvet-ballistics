#![forbid(unsafe_code)]
//! Integration tests for basic engine workflow execution.

use crate::errors::EngineError;
use crate::frame::StepState;
use crate::ids::{ConstIdx, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

use crate::engine::{
    EngineSignal, StepBudget, build_list_impl, build_object_impl, new_run_frame, run_until_blocked,
    step_once,
};

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

// ===== Basic workflow execution =====

#[test]
fn set_chain_finishes_with_slot_value() -> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::I64(42)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(7), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

    ensure_equal(
        result,
        Ok(EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)),
    )?;
    ensure_equal(run.executed(), 2)?;
    Ok(())
}

#[test]
fn set_chain_finishes_with_object_slot_value() -> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::Bool(true)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(8), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

    ensure_equal(
        result,
        Ok(EngineSignal::Finished(SlotValue::Bool(true), Taint::Clean)),
    )?;
    Ok(())
}

#[test]
fn const_finish_returns_constant_pool_value() -> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::Bool(true)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(9), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    if result == EngineSignal::Finished(SlotValue::Bool(true), Taint::Clean) {
        Ok(())
    } else {
        Err(format!("unexpected const finish result: {result:?}"))
    }
}

#[test]
fn set_const_rejects_missing_constant() -> Result<(), String> {
    let result = missing_constant_workflow(ConstIdx::new(1));

    match result {
        Err(crate::WorkflowError::ConstOutOfBounds { constant })
            if constant == ConstIdx::new(1) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected const validation result: {other:?}")),
    }
}

// ===== Copy node =====

#[test]
fn copy_preserves_value_and_taint() -> Result<(), String> {
    let workflow = copy_workflow(Some(SlotIdx::new(1))).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(18), &workflow)?;
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::I64(77),
        Taint::DerivedFromSecret,
    )
    .map_err(|error| error.to_string())?;

    let mut store = test_store();
    let signal = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    ensure_equal(run.read_slot(SlotIdx::new(1)), Ok(&SlotValue::I64(77)))?;
    ensure_equal(
        run.read_taint(SlotIdx::new(1)),
        Ok(Taint::DerivedFromSecret),
    )?;
    Ok(())
}

#[test]
fn failed_node_is_marked_failed_on_typed_error() -> Result<(), String> {
    let workflow = copy_workflow(None).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(19), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
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

// ===== Build list/object =====

#[test]
fn build_list_copies_slot_values_in_exact_item_order() -> Result<(), String> {
    let mut store = test_store();
    let mut run = crate::RunFrame::new(RunId::new(32), StepIdx::new(0), 1, 3)
        .map_err(|error| error.to_string())?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(10))
        .map_err(|error| error.to_string())?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))
        .map_err(|error| error.to_string())?;
    run.write_slot(SlotIdx::new(2), SlotValue::Null)
        .map_err(|error| error.to_string())?;

    let list = build_list_impl(
        &mut store,
        &run,
        &[SlotIdx::new(1), SlotIdx::new(0), SlotIdx::new(2)],
    )
    .map_err(|error| error.to_string())?;
    let items = store.list(list).map_err(|error| error.to_string())?;

    ensure_equal(items.len(), 3)?;
    ensure_equal(items.first().copied(), Some(SlotValue::Bool(true)))?;
    ensure_equal(items.get(1).copied(), Some(SlotValue::I64(10)))?;
    ensure_equal(items.get(2).copied(), Some(SlotValue::Null))?;
    Ok(())
}

#[test]
fn build_object_preserves_field_order_and_first_duplicate_lookup() -> Result<(), String> {
    let mut store = test_store();
    let mut run = crate::RunFrame::new(RunId::new(33), StepIdx::new(0), 1, 3)
        .map_err(|error| error.to_string())?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(100))
        .map_err(|error| error.to_string())?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(200))
        .map_err(|error| error.to_string())?;
    run.write_slot(SlotIdx::new(2), SlotValue::Bool(false))
        .map_err(|error| error.to_string())?;
    let duplicate_key = SymbolId::new(7);
    let tail_key = SymbolId::new(9);

    let object = build_object_impl(
        &mut store,
        &run,
        &[
            (duplicate_key, SlotIdx::new(0)),
            (duplicate_key, SlotIdx::new(1)),
            (tail_key, SlotIdx::new(2)),
        ],
    )
    .map_err(|error| error.to_string())?;
    let fields = store.object(object).map_err(|error| error.to_string())?;

    ensure_equal(fields.len(), 3)?;
    ensure_equal(
        fields.first().copied(),
        Some(ObjectField {
            key: duplicate_key,
            value: SlotValue::I64(100),
            taint: Taint::Clean,
        }),
    )?;
    ensure_equal(
        fields.get(1).copied(),
        Some(ObjectField {
            key: duplicate_key,
            value: SlotValue::I64(200),
            taint: Taint::Clean,
        }),
    )?;
    ensure_equal(
        fields.get(2).copied(),
        Some(ObjectField {
            key: tail_key,
            value: SlotValue::Bool(false),
            taint: Taint::Clean,
        }),
    )?;
    ensure_equal(
        store
            .object_field(object, duplicate_key)
            .map_err(|error| error.to_string())?,
        SlotValue::I64(100),
    )?;
    Ok(())
}

#[test]
fn build_list_rejects_unreadable_item_slot_without_inserting() -> Result<(), String> {
    let mut store = test_store();
    let run = crate::RunFrame::new(RunId::new(34), StepIdx::new(0), 1, 1)
        .map_err(|error| error.to_string())?;

    match build_list_impl(&mut store, &run, &[SlotIdx::new(1)]) {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(1) => {
            ensure_equal(store.list_count(), 0)
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn build_object_rejects_unreadable_field_slot_without_inserting() -> Result<(), String> {
    let mut store = test_store();
    let run = crate::RunFrame::new(RunId::new(35), StepIdx::new(0), 1, 1)
        .map_err(|error| error.to_string())?;

    match build_object_impl(&mut store, &run, &[(SymbolId::new(1), SlotIdx::new(1))]) {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(1) => {
            ensure_equal(store.object_count(), 0)
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn build_nodes_finish_with_constructed_handles() -> Result<(), String> {
    let workflow = construction_workflow().map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(36), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;
    let object = match result {
        EngineSignal::Finished(SlotValue::Object(object), Taint::Clean) => object,
        other => return Err(format!("unexpected result: {other:?}")),
    };
    let list = match store.object_field(object, SymbolId::new(1)) {
        Ok(SlotValue::List(list)) => list,
        other => return Err(format!("unexpected object field: {other:?}")),
    };
    let items = store.list(list).map_err(|error| error.to_string())?;

    ensure_equal(items.first().copied(), Some(SlotValue::I64(11)))?;
    ensure_equal(items.get(1).copied(), Some(SlotValue::I64(22)))?;
    ensure_equal(
        store.object_field(object, SymbolId::new(2)),
        Ok(SlotValue::I64(11)),
    )?;
    Ok(())
}

// ===== Workflow helpers =====

fn tiny_workflow(value: ConstValue) -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(tiny_workflow_parts(value))
}

fn tiny_workflow_parts(value: ConstValue) -> WorkflowParts {
    WorkflowParts {
        name: Box::<str>::from("tiny"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: tiny_workflow_nodes(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![value].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    }
}

fn missing_constant_workflow(constant: ConstIdx) -> Result<CompiledWorkflow, crate::WorkflowError> {
    let mut parts = tiny_workflow_parts(ConstValue::Null);
    parts.nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst { value: constant },
        },
        tiny_finish_node(),
    ]
    .into_boxed_slice();
    CompiledWorkflow::try_from_parts(parts)
}

fn tiny_workflow_nodes() -> Box<[CompiledNode]> {
    vec![tiny_set_const_node(), tiny_finish_node()].into_boxed_slice()
}

fn tiny_set_const_node() -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    }
}

fn tiny_finish_node() -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }
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
            tiny_finish_node(),
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
        input_slots: Box::new([]),    })
}

fn construction_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("construction"),
        digest: WorkflowDigest::from_bytes([0x36; 32]),
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
                output: Some(SlotIdx::new(3)),
                next: Some(StepIdx::new(4)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![
                        (SymbolId::new(1), SlotIdx::new(2)),
                        (SymbolId::new(2), SlotIdx::new(0)),
                    ]
                    .into_boxed_slice(),
                },
            },
            CompiledNode {
                id: StepIdx::new(4),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(3),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(11), ConstValue::I64(22)].into_boxed_slice(),
        slot_count: 4,
        symbols_count: 3,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
}
