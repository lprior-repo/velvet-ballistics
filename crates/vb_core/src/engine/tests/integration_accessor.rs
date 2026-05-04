//! Integration tests for accessor evaluation.

use crate::errors::EngineError;
use crate::ids::{AccessorIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::value::{SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, PathSegment,
    ResourceContract, WorkflowParts,
};

use crate::engine::{EngineSignal, StepBudget, eval_accessor, eval_accessor_with_store,
    new_run_frame, run_until_blocked};

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

#[test]
fn public_eval_accessor_loads_root_value() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(24), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
        .map_err(|error| error.to_string())?;

    let value = eval_accessor(&workflow, &run, AccessorIdx::new(0))
        .map_err(|error| error.to_string())?;

    ensure_equal(value, SlotValue::I64(77))?;
    Ok(())
}

#[test]
fn eval_accessor_identity_path_returns_root_handle_without_store() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(120), &workflow)?;
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Object(ObjectId::new(42)),
        Taint::Clean,
    )
    .map_err(|error| error.to_string())?;

    let value = eval_accessor(&workflow, &run, AccessorIdx::new(0))
        .map_err(|error| error.to_string())?;

    ensure_equal(value, SlotValue::Object(ObjectId::new(42)))?;
    Ok(())
}

#[test]
fn public_eval_accessor_rejects_invalid_accessor_index() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
    let run = test_frame(RunId::new(27), &workflow)?;

    match eval_accessor(&workflow, &run, AccessorIdx::new(1)) {
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn load_accessor_with_empty_path_loads_root_slot() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(15), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    if result == EngineSignal::Finished(SlotValue::I64(77), Taint::Clean) {
        Ok(())
    } else {
        Err(format!("unexpected result: {result:?}"))
    }
}

#[test]
fn public_eval_accessor_reports_typed_error_without_store() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice())
            .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(16), &workflow)?;
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Object(ObjectId::new(0)),
        Taint::Clean,
    )
    .map_err(|error| error.to_string())?;

    match eval_accessor(&workflow, &run, AccessorIdx::new(0)) {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "object",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn load_accessor_reads_object_field_through_store() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(7))].into_boxed_slice())
            .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(28), &workflow)?;
    let mut store = test_store();
    let object = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(7),
                value: SlotValue::I64(123),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(object), Taint::Clean)
        .map_err(|error| error.to_string())?;

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(123), Taint::Clean),
    )?;
    Ok(())
}

#[test]
fn eval_accessor_reads_list_item_through_store() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(1)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(29), &workflow)?;
    let mut store = test_store();
    let list = store
        .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|error| error.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|error| error.to_string())?;

    ensure_equal(value, SlotValue::I64(2))?;
    Ok(())
}

#[test]
fn eval_accessor_reports_missing_field_precisely() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(9))].into_boxed_slice())
            .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(30), &workflow)?;
    let mut store = test_store();
    let object = store
        .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(object), Taint::Clean)
        .map_err(|error| error.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ObjectFieldNotFound { field }) if field == SymbolId::new(9) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_reports_list_index_precisely() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(4)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(31), &workflow)?;
    let mut store = test_store();
    let list = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|error| error.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ListIndexOutOfBounds { index: 4 }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_rejects_field_traversal_on_scalar_value() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(7))].into_boxed_slice())
            .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(121), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(11), Taint::Clean)
        .map_err(|error| error.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "number",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_reports_object_handle_bounds() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(3))].into_boxed_slice())
            .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(122), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Object(ObjectId::new(99)),
        Taint::Clean,
    )
    .map_err(|error| error.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ObjectOutOfBounds { object }) if object == ObjectId::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_reports_list_handle_bounds() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(123), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::List(ListId::new(88)),
        Taint::Clean,
    )
    .map_err(|error| error.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ListOutOfBounds { list }) if list == ListId::new(88) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

fn accessor_workflow(
    path: Box<[PathSegment]>,
) -> Result<CompiledWorkflow, crate::WorkflowError> {
    let expression = ExprProgram::try_from_ops(
        vec![ExprOp::LoadAccessor(AccessorIdx::new(0))].into_boxed_slice(),
    )
    .map_err(crate::WorkflowError::Expression)?;
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("accessor"),
        digest: WorkflowDigest::from_bytes([8; 32]),
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
        accessors: vec![AccessorProgram {
            root: SlotIdx::new(0),
            path,
        }]
        .into_boxed_slice(),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 100,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
}
