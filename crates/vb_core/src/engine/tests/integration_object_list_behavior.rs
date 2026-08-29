#![forbid(unsafe_code)]
//! Integration behavior tests for object_list and node_helpers.
//! Covers ValueStore list/object CRUD, RunFrame-based build ops,
//! node helpers (set_const, copy_slot), workflow structure, error
//! paths, taint propagation, and proptest invariants.

use crate::errors::EngineError;
use crate::ids::{ConstIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};
use crate::workflow::ResourceContract;

use crate::engine::{
    EngineSignal, new_run_frame, run_until_blocked, StepBudget,
};

use proptest::prelude::*;

fn test_store() -> ValueStore {
    ValueStore::new()
}

fn test_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<crate::RunFrame, String> {
    new_run_frame(run_id, workflow).map_err(|error| error.to_string())
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

// ===== 1. Object list creation =====

#[test]
fn list_creation_empty_produces_empty_list() -> Result<(), String> {
    let mut store = test_store();
    let lid = store.insert_list(vec![].into_boxed_slice()).map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    ensure_equal(items.is_empty(), true)
}

#[test]
fn list_creation_with_three_items_preserves_all() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    ensure_equal(items.len(), 3)?;
    ensure_equal(items[0], SlotValue::I64(1))?;
    ensure_equal(items[1], SlotValue::I64(2))?;
    ensure_equal(items[2], SlotValue::I64(3))
}

#[test]
fn list_creation_single_item_stores_correctly() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![SlotValue::Bool(true)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    ensure_equal(items.len(), 1)?;
    ensure_equal(items[0], SlotValue::Bool(true))
}

#[test]
fn list_creation_mixed_types_preserves_all_variants() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![
                SlotValue::Null,
                SlotValue::Bool(false),
                SlotValue::I64(-42),
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    ensure_equal(items[0], SlotValue::Null)?;
    ensure_equal(items[1], SlotValue::Bool(false))?;
    ensure_equal(items[2], SlotValue::I64(-42))
}

#[test]
fn object_creation_empty_produces_empty_object() -> Result<(), String> {
    let mut store = test_store();
    let oid = store
        .insert_object(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let fields = store.object(oid).map_err(|e| e.to_string())?;
    ensure_equal(fields.is_empty(), true)
}

#[test]
fn object_creation_single_field_stores_correctly() -> Result<(), String> {
    let mut store = test_store();
    let key = SymbolId::new(10);
    let oid = store
        .insert_object(
            vec![ObjectField {
                key,
                value: SlotValue::I64(99),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let fields = store.object(oid).map_err(|e| e.to_string())?;
    ensure_equal(fields.len(), 1)?;
    ensure_equal(fields[0].key, key)?;
    ensure_equal(fields[0].value, SlotValue::I64(99))
}

#[test]
fn object_creation_multiple_fields_preserves_order() -> Result<(), String> {
    let mut store = test_store();
    let oid = store
        .insert_object(
            vec![
                ObjectField {
                    key: SymbolId::new(0),
                    value: SlotValue::I64(1),
                    taint: Taint::Clean,
                },
                ObjectField {
                    key: SymbolId::new(1),
                    value: SlotValue::I64(2),
                    taint: Taint::Clean,
                },
                ObjectField {
                    key: SymbolId::new(2),
                    value: SlotValue::I64(3),
                    taint: Taint::Clean,
                },
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let fields = store.object(oid).map_err(|e| e.to_string())?;
    ensure_equal(fields.len(), 3)?;
    ensure_equal(fields[0].value, SlotValue::I64(1))?;
    ensure_equal(fields[1].value, SlotValue::I64(2))?;
    ensure_equal(fields[2].value, SlotValue::I64(3))
}

// ===== 2. Object list index access =====

#[test]
fn list_index_access_valid_index_returns_correct_item() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)]
                .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    ensure_equal(
        store.list_item(lid, 1).map_err(|e| e.to_string())?,
        SlotValue::I64(20),
    )
}

#[test]
fn list_index_access_out_of_bounds_returns_error() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![SlotValue::I64(10)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    match store.list_item(lid, 5) {
        Err(crate::errors::CoreError::ListIndexOutOfBounds { index: 5 }) => Ok(()),
        other => Err(format!("expected ListIndexOutOfBounds, got {other:?}")),
    }
}

#[test]
fn list_index_access_empty_list_rejects_index_zero() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    match store.list_item(lid, 0) {
        Err(crate::errors::CoreError::ListIndexOutOfBounds { index: 0 }) => Ok(()),
        other => Err(format!("expected ListIndexOutOfBounds, got {other:?}")),
    }
}

#[test]
fn object_field_access_present_key_resolves_value() -> Result<(), String> {
    let mut store = test_store();
    let key = SymbolId::new(77);
    let oid = store
        .insert_object(
            vec![ObjectField {
                key,
                value: SlotValue::Bool(true),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    ensure_equal(
        store.object_field(oid, key).map_err(|e| e.to_string())?,
        SlotValue::Bool(true),
    )
}

#[test]
fn object_field_access_missing_key_returns_not_found() -> Result<(), String> {
    let mut store = test_store();
    let present_key = SymbolId::new(1);
    let absent_key = SymbolId::new(99);
    let oid = store
        .insert_object(
            vec![ObjectField {
                key: present_key,
                value: SlotValue::Null,
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    match store.object_field(oid, absent_key) {
        Err(crate::errors::CoreError::ObjectFieldNotFound { field }) if field == absent_key => {
            Ok(())
        }
        other => Err(format!("expected ObjectFieldNotFound, got {other:?}")),
    }
}

#[test]
fn object_field_access_out_of_bounds_object_returns_error() -> Result<(), String> {
    let store = test_store();
    match store.object_field(ObjectId::new(99), SymbolId::new(0)) {
        Err(crate::errors::CoreError::ObjectOutOfBounds { object }) if object == ObjectId::new(99) => {
            Ok(())
        }
        other => Err(format!("expected ObjectOutOfBounds, got {other:?}")),
    }
}

#[test]
fn list_index_access_out_of_bounds_list_returns_error() -> Result<(), String> {
    let store = test_store();
    match store.list_item(ListId::new(99), 0) {
        Err(crate::errors::CoreError::ListOutOfBounds { list }) if list == ListId::new(99) => Ok(()),
        other => Err(format!("expected ListOutOfBounds, got {other:?}")),
    }
}

// ===== 3. Iteration =====

#[test]
fn list_forward_iteration_yields_all_items_in_order() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let collected: Vec<SlotValue> = items.iter().copied().collect();
    ensure_equal(
        collected,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    )
}

#[test]
fn list_reverse_iteration_yields_all_items_reversed() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let collected: Vec<SlotValue> = items.iter().rev().copied().collect();
    ensure_equal(
        collected,
        vec![SlotValue::I64(3), SlotValue::I64(2), SlotValue::I64(1)],
    )
}

#[test]
fn list_iteration_empty_yields_no_items() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    ensure_equal(items.iter().count(), 0)
}

#[test]
fn object_iteration_preserves_field_insertion_order() -> Result<(), String> {
    let mut store = test_store();
    let oid = store
        .insert_object(
            vec![
                ObjectField {
                    key: SymbolId::new(10),
                    value: SlotValue::I64(100),
                    taint: Taint::Clean,
                },
                ObjectField {
                    key: SymbolId::new(20),
                    value: SlotValue::I64(200),
                    taint: Taint::Clean,
                },
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let fields = store.object(oid).map_err(|e| e.to_string())?;
    ensure_equal(fields.len(), 2)?;
    ensure_equal(fields[0].key, SymbolId::new(10))?;
    ensure_equal(fields[1].key, SymbolId::new(20))
}

// ===== 4. Length/Count =====

#[test]
fn list_length_empty_returns_zero() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    ensure_equal(store.list(lid).map_err(|e| e.to_string())?.len(), 0)
}

#[test]
fn list_length_single_returns_one() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![SlotValue::Null].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    ensure_equal(store.list(lid).map_err(|e| e.to_string())?.len(), 1)
}

#[test]
fn list_length_many_returns_correct_count() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(0); 42].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    ensure_equal(store.list(lid).map_err(|e| e.to_string())?.len(), 42)
}

#[test]
fn object_length_empty_returns_zero() -> Result<(), String> {
    let mut store = test_store();
    let oid = store
        .insert_object(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    ensure_equal(store.object(oid).map_err(|e| e.to_string())?.len(), 0)
}

#[test]
fn object_length_multiple_returns_correct_count() -> Result<(), String> {
    let mut store = test_store();
    let fields: Vec<ObjectField> = (0..7)
        .map(|i| ObjectField {
            key: SymbolId::new(i),
            value: SlotValue::I64(i as i64),
            taint: Taint::Clean,
        })
        .collect();
    let oid = store
        .insert_object(fields.into_boxed_slice())
        .map_err(|e| e.to_string())?;
    ensure_equal(store.object(oid).map_err(|e| e.to_string())?.len(), 7)
}

// ===== 5. First/Last =====

#[test]
fn list_first_non_empty_returns_first_element() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(5), SlotValue::I64(6)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    ensure_equal(
        store.list(lid).map_err(|e| e.to_string())?.first(),
        Some(&SlotValue::I64(5)),
    )
}

#[test]
fn list_last_non_empty_returns_last_element() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(5), SlotValue::I64(6)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    ensure_equal(
        store.list(lid).map_err(|e| e.to_string())?.last(),
        Some(&SlotValue::I64(6)),
    )
}

#[test]
fn list_first_empty_returns_none() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    ensure_equal(store.list(lid).map_err(|e| e.to_string())?.first(), None)
}

#[test]
fn list_last_empty_returns_none() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    ensure_equal(store.list(lid).map_err(|e| e.to_string())?.last(), None)
}

#[test]
fn list_single_element_first_and_last_are_same() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![SlotValue::I64(42)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    ensure_equal(items.first(), Some(&SlotValue::I64(42)))?;
    ensure_equal(items.last(), Some(&SlotValue::I64(42)))
}

// ===== 6. Filter/Predicate =====

fn is_positive(v: &SlotValue) -> bool {
    matches!(v, SlotValue::I64(n) if *n > 0)
}

#[test]
fn list_filter_keeps_matching_elements() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(-1), SlotValue::I64(0), SlotValue::I64(3), SlotValue::I64(-4)]
                .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let positive: Vec<SlotValue> = items.iter().copied().filter(|v| is_positive(v)).collect();
    ensure_equal(positive, vec![SlotValue::I64(3)])
}

#[test]
fn list_filter_all_rejected_returns_empty_collection() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(-1), SlotValue::I64(-2)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let positive: Vec<SlotValue> = items.iter().copied().filter(|v| is_positive(v)).collect();
    ensure_equal(positive.is_empty(), true)
}

#[test]
fn list_filter_all_accepted_returns_all_elements() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let positive: Vec<SlotValue> = items.iter().copied().filter(|v| is_positive(v)).collect();
    ensure_equal(positive.len(), 3)
}

#[test]
fn list_filter_on_empty_returns_empty() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let positive: Vec<SlotValue> = items.iter().copied().filter(|v| is_positive(v)).collect();
    ensure_equal(positive.is_empty(), true)
}

// ===== 7. Map/Transform =====

fn double_i64(v: &SlotValue) -> SlotValue {
    match v {
        SlotValue::I64(n) => SlotValue::I64(n * 2),
        other => *other,
    }
}

#[test]
fn list_map_transforms_all_elements() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let doubled: Vec<SlotValue> = items.iter().map(|v| double_i64(v)).collect();
    ensure_equal(
        doubled,
        vec![SlotValue::I64(2), SlotValue::I64(4), SlotValue::I64(6)],
    )
}

#[test]
fn list_map_empty_produces_empty() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let doubled: Vec<SlotValue> = items.iter().map(|v| double_i64(v)).collect();
    ensure_equal(doubled.is_empty(), true)
}

#[test]
fn list_map_preserves_original_unchanged() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let _doubled: Vec<SlotValue> = store
        .list(lid)
        .map_err(|e| e.to_string())?
        .iter()
        .map(|v| double_i64(v))
        .collect();
    let items = store.list(lid).map_err(|e| e.to_string())?;
    ensure_equal(items[0], SlotValue::I64(1))?;
    ensure_equal(items[1], SlotValue::I64(2))
}

// ===== 8. Sort =====

#[test]
fn list_sort_ascending_reorders_elements() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(3), SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let mut sorted: Vec<SlotValue> = items.to_vec();
    sorted.sort_by(|a, b| {
        let na = if let SlotValue::I64(n) = a { *n } else { 0 };
        let nb = if let SlotValue::I64(n) = b { *n } else { 0 };
        na.cmp(&nb)
    });
    ensure_equal(
        sorted,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    )
}

#[test]
fn list_sort_empty_does_not_panic() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let mut sorted: Vec<SlotValue> = items.to_vec();
    sorted.sort_by(|a, b| {
        let na = if let SlotValue::I64(n) = a { *n } else { 0 };
        let nb = if let SlotValue::I64(n) = b { *n } else { 0 };
        na.cmp(&nb)
    });
    ensure_equal(sorted.is_empty(), true)
}

// ===== 9. Dedup =====

#[test]
fn list_dedup_removes_consecutive_duplicates() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(2), SlotValue::I64(3)]
                .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let mut deduped: Vec<SlotValue> = items.to_vec();
    deduped.dedup();
    ensure_equal(
        deduped,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    )
}

#[test]
fn list_dedup_no_duplicates_preserves_all() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let mut deduped: Vec<SlotValue> = items.to_vec();
    deduped.dedup();
    ensure_equal(deduped.len(), 3)
}

#[test]
fn list_dedup_empty_does_not_panic() -> Result<(), String> {
    let mut store = test_store();
    let lid = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let items = store.list(lid).map_err(|e| e.to_string())?;
    let mut deduped: Vec<SlotValue> = items.to_vec();
    deduped.dedup();
    ensure_equal(deduped.is_empty(), true)
}

// ===== 10. Node helpers =====

fn set_const_workflow(value: ConstValue) -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("set_const_test"),
        digest: WorkflowDigest::from_bytes([0xAA; 32]),
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
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
}

#[test]
fn set_const_i64_writes_to_slot_and_finishes() -> Result<(), String> {
    let workflow =
        set_const_workflow(ConstValue::I64(99)).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(1), &workflow)?;
    let mut store = test_store();
    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
    ensure_equal(
        result,
        Ok(EngineSignal::Finished(SlotValue::I64(99), Taint::Clean)),
    )
}

#[test]
fn set_const_null_writes_null_and_finishes() -> Result<(), String> {
    let workflow =
        set_const_workflow(ConstValue::Null).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(5), &workflow)?;
    let mut store = test_store();
    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
    ensure_equal(
        result,
        Ok(EngineSignal::Finished(SlotValue::Null, Taint::Clean)),
    )
}

#[test]
fn set_const_bool_writes_bool_and_finishes() -> Result<(), String> {
    let workflow =
        set_const_workflow(ConstValue::Bool(false)).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(6), &workflow)?;
    let mut store = test_store();
    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
    ensure_equal(
        result,
        Ok(EngineSignal::Finished(SlotValue::Bool(false), Taint::Clean)),
    )
}

fn copy_slot_taint_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("copy_slot_test"),
        digest: WorkflowDigest::from_bytes([0xBB; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
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
                    result: SlotIdx::new(1),
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
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
}

#[test]
fn copy_slot_copies_value_and_taint_correctly() -> Result<(), String> {
    let workflow =
        copy_slot_taint_workflow()
            .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(10), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;
    let mut store = test_store();
    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
    ensure_equal(
        result,
        Ok(EngineSignal::Finished(
            SlotValue::I64(77),
            Taint::DerivedFromSecret,
        )),
    )
}

#[test]
fn multi_step_progression_accumulates_executed_count() -> Result<(), String> {
    let workflow =
        set_const_workflow(ConstValue::I64(42)).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(11), &workflow)?;
    let mut store = test_store();
    let _ = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;
    ensure_equal(run.executed(), 2)
}

#[test]
fn copy_slot_clean_taint_preserves_clean() -> Result<(), String> {
    let workflow =
        copy_slot_taint_workflow()
            .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(12), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    let mut store = test_store();
    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
    ensure_equal(
        result,
        Ok(EngineSignal::Finished(SlotValue::I64(1), Taint::Clean)),
    )
}

// ===== 11. Node type detection =====

#[test]
fn node_kind_detects_set_const() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    assert!(matches!(node.kind, CompiledNodeKind::SetConst { .. }));
}

#[test]
fn node_kind_detects_finish() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    assert!(matches!(node.kind, CompiledNodeKind::Finish { .. }));
}

#[test]
fn node_kind_detects_copy() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
    };
    assert!(matches!(node.kind, CompiledNodeKind::Copy { .. }));
}

#[test]
fn node_kind_detects_build_list() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::BuildList {
            items: Box::new([]),
        },
    };
    assert!(matches!(node.kind, CompiledNodeKind::BuildList { .. }));
}

#[test]
fn node_kind_detects_build_object() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::BuildObject {
            fields: Box::new([]),
        },
    };
    assert!(matches!(node.kind, CompiledNodeKind::BuildObject { .. }));
}

#[test]
fn node_kind_detects_nop() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    assert!(matches!(node.kind, CompiledNodeKind::Nop));
}

// ===== 12. Workflow chain/graph =====

fn three_node_chain_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("chain_test"),
        digest: WorkflowDigest::from_bytes([0xCC; 32]),
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
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
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
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(101)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
}

#[test]
fn workflow_chain_executes_all_children_in_sequence() -> Result<(), String> {
    let workflow = three_node_chain_workflow().map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(20), &workflow)?;
    let mut store = test_store();
    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
    ensure_equal(
        result,
        Ok(EngineSignal::Finished(SlotValue::I64(101), Taint::Clean)),
    )
}

#[test]
fn workflow_node_ids_are_monotonic() -> Result<(), String> {
    let workflow = three_node_chain_workflow().map_err(|e| e.to_string())?;
    let count = workflow.node_count() as usize;
    ensure_equal(count >= 3, true)?;
    for i in 1..count {
        let prev = workflow.node(StepIdx::new((i - 1) as u16));
        let curr = workflow.node(StepIdx::new(i as u16));
        match (prev, curr) {
            (Some(p), Some(c)) => {
                if c.id.get() <= p.id.get() {
                    return Err(format!(
                        "node ids not monotonic: {} then {}",
                        p.id.get(),
                        c.id.get()
                    ));
                }
            }
            _ => return Err(format!("missing node at index {i}")),
        }
    }
    Ok(())
}

#[test]
fn workflow_entry_is_first_node() -> Result<(), String> {
    let workflow = three_node_chain_workflow().map_err(|e| e.to_string())?;
    ensure_equal(workflow.entry(), StepIdx::new(0))
}

#[test]
fn workflow_depth_equals_node_count_for_linear_chain() -> Result<(), String> {
    let workflow = three_node_chain_workflow().map_err(|e| e.to_string())?;
    ensure_equal(workflow.node_count(), 3)
}

#[test]
fn workflow_children_reachable_via_next_pointers() -> Result<(), String> {
    let workflow = three_node_chain_workflow().map_err(|e| e.to_string())?;
    let parts = workflow.to_parts();
    ensure_equal(parts.nodes[0].next, Some(StepIdx::new(1)))?;
    ensure_equal(parts.nodes[1].next, Some(StepIdx::new(2)))?;
    ensure_equal(parts.nodes[2].next, None)
}

// ===== 13. Error paths =====

fn missing_output_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("missing_output"),
        digest: WorkflowDigest::from_bytes([0xDD; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
}

#[test]
fn error_on_missing_output_returns_missing_output_slot() -> Result<(), String> {
    let workflow = missing_output_workflow().map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(30), &workflow)?;
    let mut store = test_store();
    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
    match result {
        Err(EngineError::MissingOutputSlot { step }) if step == StepIdx::new(0) => Ok(()),
        other => Err(format!("expected MissingOutputSlot, got {other:?}")),
    }
}

fn const_oob_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("const_oob"),
        digest: WorkflowDigest::from_bytes([0xEE; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(5),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
}

#[test]
fn error_on_const_out_of_bounds_within_frame() -> Result<(), String> {
    let workflow = const_oob_workflow().map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(31), &workflow)?;
    let mut store = test_store();
    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
    match result {
        Err(EngineError::ConstOutOfBounds { index }) if index == ConstIdx::new(5) => Ok(()),
        other => Err(format!("expected ConstOutOfBounds, got {other:?}")),
    }
}

#[test]
fn finish_taint_secret_propagates_to_signal() -> Result<(), String> {
    let workflow =
        copy_slot_taint_workflow()
            .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(50), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(33), Taint::Secret)
        .map_err(|e| e.to_string())?;
    let mut store = test_store();
    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
    ensure_equal(
        result,
        Ok(EngineSignal::Finished(SlotValue::I64(33), Taint::Secret)),
    )
}

#[test]
fn finish_taint_derived_from_secret_propagates() -> Result<(), String> {
    let workflow =
        copy_slot_taint_workflow()
            .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(51), &workflow)?;
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )
    .map_err(|e| e.to_string())?;
    let mut store = test_store();
    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
    ensure_equal(
        result,
        Ok(EngineSignal::Finished(SlotValue::Bool(true), Taint::DerivedFromSecret)),
    )
}

#[test]
fn finish_on_uninitialized_slot_returns_error() -> Result<(), String> {
    let workflow =
        copy_slot_taint_workflow()
            .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(52), &workflow)?;
    let mut store = test_store();
    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);
    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(0) => Ok(()),
        other => Err(format!("expected SlotUninitialized, got {other:?}")),
    }
}

// ===== 14. Proptest invariants =====

proptest! {
    #[test]
    fn list_append_get_roundtrip_preserves_values(
        values in prop::collection::vec(any::<i64>(), 0..100)
    ) {
        let mut store = ValueStore::new();
        let slots: Vec<SlotValue> = values.iter().map(|v| SlotValue::I64(*v)).collect();
        let lid = store
            .insert_list(slots.clone().into_boxed_slice())
            .expect("insert list");
        let retrieved = store.list(lid).expect("list access");
        prop_assert_eq!(retrieved, slots.as_slice());
    }

    #[test]
    fn list_filter_length_invariant_preserves_subset_semantics(
        values in prop::collection::vec(any::<i64>(), 0..100)
    ) {
        let mut store = ValueStore::new();
        let slots: Vec<SlotValue> = values.iter().map(|v| SlotValue::I64(*v)).collect();
        let lid = store
            .insert_list(slots.into_boxed_slice())
            .expect("insert list");
        let items = store.list(lid).expect("list access");
        let filtered: Vec<_> = items.iter().filter(|v| {
            matches!(v, SlotValue::I64(n) if *n > 0)
        }).collect();
        prop_assert!(filtered.len() <= items.len());
    }

    #[test]
    fn list_length_matches_inserted_count(
        values in prop::collection::vec(any::<i64>(), 0..100)
    ) {
        let mut store = ValueStore::new();
        let slots: Vec<SlotValue> = values.iter().map(|v| SlotValue::I64(*v)).collect();
        let count = slots.len();
        let lid = store
            .insert_list(slots.into_boxed_slice())
            .expect("insert list");
        let items = store.list(lid).expect("list access");
        prop_assert_eq!(items.len(), count);
    }

    #[test]
    fn list_double_reverse_is_identity(
        values in prop::collection::vec(any::<i64>(), 0..100)
    ) {
        let mut store = ValueStore::new();
        let slots: Vec<SlotValue> = values.iter().map(|v| SlotValue::I64(*v)).collect();
        let lid = store
            .insert_list(slots.clone().into_boxed_slice())
            .expect("insert list");
        let items = store.list(lid).expect("list access");
        let mut reversed: Vec<SlotValue> = items.iter().copied().rev().collect();
        reversed.reverse();
        prop_assert_eq!(reversed, slots);
    }

    #[test]
    fn list_first_after_append_is_always_first_inserted(
        values in prop::collection::vec(any::<i64>(), 1..100)
    ) {
        let mut store = ValueStore::new();
        let slots: Vec<SlotValue> = values.iter().map(|v| SlotValue::I64(*v)).collect();
        let lid = store
            .insert_list(slots.into_boxed_slice())
            .expect("insert list");
        let items = store.list(lid).expect("list access");
        prop_assert_eq!(items.first(), Some(&SlotValue::I64(values[0])));
    }
}
