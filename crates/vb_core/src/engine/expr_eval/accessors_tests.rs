#![forbid(unsafe_code)]
//! Accessor evaluation tests.

use crate::engine::expr_eval::stack::{pop_value, ExprStack};
use crate::errors::EngineError;
use crate::ids::{AccessorIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::value::{SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, AccessorProgram, PathSegment,
    ResourceContract, WorkflowParts,
};

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

fn accessor_workflow(path: Box<[PathSegment]>) -> Result<CompiledWorkflow, String> {
    accessor_workflow_with_symbols(path, 10)
}

fn accessor_workflow_with_symbols(
    path: Box<[PathSegment]>,
    symbols_count: u32,
) -> Result<CompiledWorkflow, String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path,
    };
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("accessor_test"),
        digest: WorkflowDigest::from_bytes([0xFE; 32]),
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
        accessors: vec![accessor].into_boxed_slice(),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn test_frame() -> Result<crate::frame::RunFrame, String> {
    crate::frame::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2).map_err(|e| e.to_string())
}

// ===== Empty path returns root value =====

#[test]
fn eval_accessor_empty_path_returns_root_slot() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([]))?;
    let mut run = test_frame()?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
        .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let value =
        crate::engine::expr_eval::accessors::eval_accessor_with_store(
            &workflow,
            &run,
            &mut store,
            AccessorIdx::new(0),
        )
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(42))
}

#[test]
fn eval_accessor_without_store_empty_path_returns_root() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([]))?;
    let mut run = test_frame()?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
        .map_err(|e| e.to_string())?;

    let value = crate::engine::expr_eval::accessors::eval_accessor(&workflow, &run, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Bool(true))
}

// ===== Field traversal on objects =====

#[test]
fn eval_accessor_object_field_traversal() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(3))].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(3),
                value: SlotValue::I64(123),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(0), SlotValue::Object(obj))
        .map_err(|e| e.to_string())?;

    let value =
        crate::engine::expr_eval::accessors::eval_accessor_with_store(
            &workflow,
            &run,
            &mut store,
            AccessorIdx::new(0),
        )
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(123))
}

// ===== Index traversal on lists =====

#[test]
fn eval_accessor_list_index_traversal() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(10), SlotValue::I64(20)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(list))
        .map_err(|e| e.to_string())?;

    let value =
        crate::engine::expr_eval::accessors::eval_accessor_with_store(
            &workflow,
            &run,
            &mut store,
            AccessorIdx::new(0),
        )
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(10))
}

#[test]
fn eval_accessor_list_second_index() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(1)].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::Bool(false), SlotValue::Bool(true)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(list))
        .map_err(|e| e.to_string())?;

    let value =
        crate::engine::expr_eval::accessors::eval_accessor_with_store(
            &workflow,
            &run,
            &mut store,
            AccessorIdx::new(0),
        )
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Bool(true))
}

// ===== Multi-segment path =====

#[test]
fn eval_accessor_multi_segment_path() -> Result<(), String> {
    // Object with field -> list at index 0
    let workflow = accessor_workflow(
        vec![PathSegment::Field(SymbolId::new(5)), PathSegment::Index(0)].into_boxed_slice(),
    )?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let inner_list = store
        .insert_list(vec![SlotValue::I64(777)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(5),
                value: SlotValue::List(inner_list),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(0), SlotValue::Object(obj))
        .map_err(|e| e.to_string())?;

    let value =
        crate::engine::expr_eval::accessors::eval_accessor_with_store(
            &workflow,
            &run,
            &mut store,
            AccessorIdx::new(0),
        )
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(777))
}

// ===== Error cases =====

#[test]
fn eval_accessor_rejects_field_on_scalar() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(1))].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
        .map_err(|e| e.to_string())?;

    let result = crate::engine::expr_eval::accessors::eval_accessor_with_store(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    );
    match result {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "number",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_rejects_index_on_scalar() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
        .map_err(|e| e.to_string())?;

    let result = crate::engine::expr_eval::accessors::eval_accessor_with_store(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    );
    match result {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "index",
            found: "boolean",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_rejects_field_on_list() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(list))
        .map_err(|e| e.to_string())?;

    let result = crate::engine::expr_eval::accessors::eval_accessor_with_store(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    );
    match result {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "list",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_rejects_index_on_object() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(0),
                value: SlotValue::I64(1),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(0), SlotValue::Object(obj))
        .map_err(|e| e.to_string())?;

    let result = crate::engine::expr_eval::accessors::eval_accessor_with_store(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    );
    match result {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "index",
            found: "object",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_missing_object_field_returns_error() -> Result<(), String> {
    // Use SymbolId(7) instead of 99 to stay within symbols_count
    let workflow = accessor_workflow_with_symbols(
        vec![PathSegment::Field(SymbolId::new(7))].into_boxed_slice(),
        10,
    )?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let obj = store
        .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(0), SlotValue::Object(obj))
        .map_err(|e| e.to_string())?;

    let result = crate::engine::expr_eval::accessors::eval_accessor_with_store(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    );
    match result {
        Err(EngineError::ObjectFieldNotFound { field }) if field == SymbolId::new(7) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_list_index_out_of_bounds_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(5)].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(list))
        .map_err(|e| e.to_string())?;

    let result = crate::engine::expr_eval::accessors::eval_accessor_with_store(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    );
    match result {
        Err(EngineError::ListIndexOutOfBounds { index: 5 }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_invalid_index_returns_error() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(3))].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(0), SlotValue::Object(ObjectId::new(99)))
        .map_err(|e| e.to_string())?;

    let result = crate::engine::expr_eval::accessors::eval_accessor_with_store(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    );
    match result {
        Err(EngineError::ObjectOutOfBounds { object }) if object == ObjectId::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_invalid_list_handle_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(0), SlotValue::List(ListId::new(88)))
        .map_err(|e| e.to_string())?;

    let result = crate::engine::expr_eval::accessors::eval_accessor_with_store(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    );
    match result {
        Err(EngineError::ListOutOfBounds { list }) if list == ListId::new(88) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_out_of_bounds_accessor_index_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([]))?;
    let run = test_frame()?;

    let result =
        crate::engine::expr_eval::accessors::eval_accessor(&workflow, &run, AccessorIdx::new(1));
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason })
            if reason.contains("accessor index out of bounds") =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ===== Without-store path traversal error =====

#[test]
fn eval_accessor_without_store_non_empty_path_returns_traversal_error() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice())?;
    let mut run = test_frame()?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
        .map_err(|e| e.to_string())?;

    let result =
        crate::engine::expr_eval::accessors::eval_accessor(&workflow, &run, AccessorIdx::new(0));
    match result {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "number",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ===== eval_accessor_with_taint_inner coverage =====

#[test]
fn eval_accessor_with_taint_empty_path_returns_root_and_taint() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([]))?;
    let mut run = test_frame()?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret)
        .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let (value, taint) = crate::engine::expr_eval::accessors::eval_accessor_with_taint_inner(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(42))?;
    ensure_equal(taint, Taint::Secret)
}

#[test]
fn eval_accessor_with_taint_object_field_traversal() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(3))].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let obj = store
        .insert_object(
            vec![ObjectField::with_taint(
                SymbolId::new(3),
                SlotValue::I64(123),
                Taint::DerivedFromSecret,
            )]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let (value, taint) = crate::engine::expr_eval::accessors::eval_accessor_with_taint_inner(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(123))?;
    ensure_equal(taint, Taint::DerivedFromSecret)
}

#[test]
fn eval_accessor_with_taint_list_index_traversal() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let list = store
        .insert_list_with_taint(
            vec![SlotValue::I64(10)].into_boxed_slice(),
            vec![Taint::Secret].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let (value, taint) = crate::engine::expr_eval::accessors::eval_accessor_with_taint_inner(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(10))?;
    ensure_equal(taint, Taint::Secret)
}

#[test]
fn eval_accessor_with_taint_multi_segment_joins_taints() -> Result<(), String> {
    let workflow = accessor_workflow(
        vec![PathSegment::Field(SymbolId::new(5)), PathSegment::Index(0)].into_boxed_slice(),
    )?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let inner_list = store
        .insert_list_with_taint(
            vec![SlotValue::I64(777)].into_boxed_slice(),
            vec![Taint::Secret].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let obj = store
        .insert_object(
            vec![ObjectField::with_taint(
                SymbolId::new(5),
                SlotValue::List(inner_list),
                Taint::DerivedFromSecret,
            )]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let (value, taint) = crate::engine::expr_eval::accessors::eval_accessor_with_taint_inner(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(0),
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(777))?;
    // Taint should be joined: Clean -> DerivedFromSecret -> Secret
    ensure_equal(taint, Taint::Secret)
}

#[test]
fn eval_accessor_with_taint_out_of_bounds_accessor_index_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([]))?;
    let run = test_frame()?;
    let mut store = ValueStore::new();

    let result = crate::engine::expr_eval::accessors::eval_accessor_with_taint_inner(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(1),
    );
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason })
            if reason.contains("accessor index out of bounds") =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ===== eval_load_accessor coverage =====

#[test]
fn eval_load_accessor_pushes_value_and_accumulates_taint() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Field(SymbolId::new(3))].into_boxed_slice())?;
    let mut run = test_frame()?;
    let mut store = ValueStore::new();
    let obj = store
        .insert_object(
            vec![ObjectField::clean(SymbolId::new(3), SlotValue::I64(42))].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let mut stack = ExprStack::new(4).map_err(|e| e.to_string())?;
    let mut taint_accum = Taint::Clean;
    crate::engine::expr_eval::accessors::eval_load_accessor(
        &workflow,
        &run,
        &mut store,
        &mut stack,
        AccessorIdx::new(0),
        &mut taint_accum,
    )
    .map_err(|e| e.to_string())?;
    let popped = pop_value(&mut stack).map_err(|e| e.to_string())?;
    ensure_equal(popped, SlotValue::I64(42))?;
    ensure_equal(taint_accum, Taint::Clean)
}

#[test]
fn eval_accessor_inner_out_of_bounds_accessor_index_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([]))?;
    let run = test_frame()?;
    let mut store = ValueStore::new();

    let result = crate::engine::expr_eval::accessors::eval_accessor_inner(
        &workflow,
        &run,
        &mut store,
        AccessorIdx::new(1),
    );
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason })
            if reason.contains("accessor index out of bounds") =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}
