//! Accessor evaluation.

use crate::errors::EngineError;
use crate::ids::AccessorIdx;
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{AccessorProgram, CompiledWorkflow};

fn eval_accessor_program_without_store(
    run: &crate::RunFrame,
    program: &AccessorProgram,
) -> Result<SlotValue, EngineError> {
    let current = *run.read_slot(program.root)?;
    if program.path.is_empty() {
        return Ok(current);
    }
    let segment = program.path.first().copied().ok_or({
        EngineError::InternalInvariantViolation {
            reason: "accessor path checked non-empty",
        }
    })?;
    Err(EngineError::UnsupportedAccessorTraversal {
        segment: path_segment_name(segment),
        found: current.type_name(),
    })
}

pub(super) fn eval_accessor_program(
    run: &crate::RunFrame,
    store: &mut ValueStore,
    program: &AccessorProgram,
) -> Result<SlotValue, EngineError> {
    let mut current = *run.read_slot(program.root)?;
    if program.path.is_empty() {
        return Ok(current);
    }

    let mut index = 0usize;
    while index < program.path.len() {
        let segment = program.path.get(index).copied().ok_or({
            EngineError::InternalInvariantViolation {
                reason: "accessor path index checked by loop bound",
            }
        })?;
        current = traverse_accessor_segment(store, current, segment)?;
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "accessor path index overflow",
            })?;
    }
    Ok(current)
}

fn traverse_accessor_segment(
    store: &ValueStore,
    current: SlotValue,
    segment: crate::workflow::PathSegment,
) -> Result<SlotValue, EngineError> {
    match (current, segment) {
        (SlotValue::Object(object), crate::workflow::PathSegment::Field(field)) => {
            store.object_field(object, field)
        }
        (SlotValue::List(list), crate::workflow::PathSegment::Index(index)) => {
            store.list_item(list, index)
        }
        (value, segment) => Err(EngineError::UnsupportedAccessorTraversal {
            segment: path_segment_name(segment),
            found: value.type_name(),
        }),
    }
}

const fn path_segment_name(segment: crate::workflow::PathSegment) -> &'static str {
    match segment {
        crate::workflow::PathSegment::Field(_) => "field",
        crate::workflow::PathSegment::Index(_) => "index",
    }
}

/// Taint-aware segment traversal. Returns the resolved value and its stored taint.
fn traverse_accessor_segment_with_taint(
    store: &ValueStore,
    current: SlotValue,
    segment: crate::workflow::PathSegment,
) -> Result<(SlotValue, crate::value::Taint), EngineError> {
    match (current, segment) {
        (SlotValue::Object(object), crate::workflow::PathSegment::Field(field)) => {
            store.object_field_with_taint(object, field)
        }
        (SlotValue::List(list), crate::workflow::PathSegment::Index(index)) => {
            store.list_item_with_taint(list, index)
        }
        (value, segment) => Err(EngineError::UnsupportedAccessorTraversal {
            segment: path_segment_name(segment),
            found: value.type_name(),
        }),
    }
}

// ===== Public API =====

pub(super) fn eval_accessor_inner(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    accessor: AccessorIdx,
) -> Result<SlotValue, EngineError> {
    let program = plan
        .accessor(accessor)
        .ok_or(EngineError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        })?;
    eval_accessor_program(run, store, program)
}

pub(super) fn eval_accessor_with_taint_inner(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    accessor: AccessorIdx,
) -> Result<(SlotValue, crate::value::Taint), EngineError> {
    let program = plan
        .accessor(accessor)
        .ok_or(EngineError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        })?;
    let mut accumulated_taint = run.read_taint(program.root)?;
    let mut current = *run.read_slot(program.root)?;

    if program.path.is_empty() {
        return Ok((current, accumulated_taint));
    }

    let mut index = 0usize;
    while index < program.path.len() {
        let segment = program.path.get(index).copied().ok_or({
            EngineError::InternalInvariantViolation {
                reason: "accessor path index checked by loop bound",
            }
        })?;
        let (value, segment_taint) = traverse_accessor_segment_with_taint(store, current, segment)?;
        accumulated_taint = crate::value::join_taint(accumulated_taint, segment_taint);
        current = value;
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "accessor path index overflow",
            })?;
    }
    Ok((current, accumulated_taint))
}

pub(super) fn eval_load_accessor(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    stack: &mut super::stack::ExprStack,
    accessor: AccessorIdx,
    taint_accum: &mut crate::value::Taint,
) -> Result<(), EngineError> {
    let (value, accessor_taint) = eval_accessor_with_taint_inner(plan, run, store, accessor)?;
    *taint_accum = crate::value::join_taint(*taint_accum, accessor_taint);
    super::stack::push_value(stack, value)
}

pub fn eval_accessor(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    accessor: AccessorIdx,
) -> Result<SlotValue, EngineError> {
    let program = plan
        .accessor(accessor)
        .ok_or(EngineError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        })?;
    eval_accessor_program_without_store(run, program)
}

pub fn eval_accessor_with_store(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    accessor: AccessorIdx,
) -> Result<SlotValue, EngineError> {
    eval_accessor_inner(plan, run, store, accessor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
    use crate::value::{SlotValue, Taint};
    use crate::value_store::{ObjectField, ValueStore};
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, PathSegment, ResourceContract,
        WorkflowParts,
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

        let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
            .map_err(|e| e.to_string())?;
        ensure_equal(value, SlotValue::I64(42))
    }

    #[test]
    fn eval_accessor_without_store_empty_path_returns_root() -> Result<(), String> {
        let workflow = accessor_workflow(Box::new([]))?;
        let mut run = test_frame()?;
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
            .map_err(|e| e.to_string())?;

        let value =
            eval_accessor(&workflow, &run, AccessorIdx::new(0)).map_err(|e| e.to_string())?;
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

        let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
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

        let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
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

        let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
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

        let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
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

        let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
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

        let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
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

        let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
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

        let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
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

        let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
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

        let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
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

        let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
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

        let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
        match result {
            Err(EngineError::ListOutOfBounds { list }) if list == ListId::new(88) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_accessor_out_of_bounds_accessor_index_returns_error() -> Result<(), String> {
        let workflow = accessor_workflow(Box::new([]))?;
        let run = test_frame()?;

        let result = eval_accessor(&workflow, &run, AccessorIdx::new(1));
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

        let result = eval_accessor(&workflow, &run, AccessorIdx::new(0));
        match result {
            Err(EngineError::UnsupportedAccessorTraversal {
                segment: "field",
                found: "number",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }
}
