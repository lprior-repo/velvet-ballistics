//! Tests for object_list module.

use vb_core::errors::EngineError;
use vb_core::ids::{RunId, SlotIdx, StepIdx, SymbolId};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;

use vb_core::engine::{build_list, build_list_with_taint, build_object, build_object_with_taint};

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

fn test_frame(slot_count: u16) -> Result<vb_core::frame::RunFrame, String> {
    vb_core::frame::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, slot_count)
        .map_err(|e| e.to_string())
}

// ===== build_object tests =====

#[test]
fn build_object_empty_fields_creates_empty_object() -> Result<(), String> {
    let mut store = ValueStore::new();
    let run = test_frame(1)?;
    let obj = build_object(&mut store, &run, &[]).map_err(|e| e.to_string())?;
    let fields = store.object(obj).map_err(|e| e.to_string())?;
    ensure_equal(fields.is_empty(), true)
}

#[test]
fn build_object_single_field_reads_correct_slot() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(10))
        .map_err(|e| e.to_string())?;
    let obj = build_object(&mut store, &run, &[(SymbolId::new(5), SlotIdx::new(0))])
        .map_err(|e| e.to_string())?;
    let fields = store.object(obj).map_err(|e| e.to_string())?;
    ensure_equal(fields.len(), 1)?;
    ensure_equal(fields[0].key, SymbolId::new(5))?;
    ensure_equal(fields[0].value, SlotValue::I64(10))
}

#[test]
fn build_object_rejects_out_of_bounds_slot() -> Result<(), String> {
    let mut store = ValueStore::new();
    let run = test_frame(1)?;
    let result = build_object(&mut store, &run, &[(SymbolId::new(0), SlotIdx::new(5))]);
    match result {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => {
            ensure_equal(store.object_count(), 0)
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ===== build_object_with_taint tests =====

#[test]
fn build_object_with_taint_all_clean_produces_clean() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(2))
        .map_err(|e| e.to_string())?;
    let (obj, taint) = build_object_with_taint(
        &mut store,
        &run,
        &[
            (SymbolId::new(0), SlotIdx::new(0)),
            (SymbolId::new(1), SlotIdx::new(1)),
        ],
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Clean)?;
    let fields = store.object(obj).map_err(|e| e.to_string())?;
    ensure_equal(fields.len(), 2)
}

#[test]
fn build_object_with_taint_joins_secret_from_one_field() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret)
        .map_err(|e| e.to_string())?;
    let (_obj, taint) = build_object_with_taint(
        &mut store,
        &run,
        &[
            (SymbolId::new(0), SlotIdx::new(0)),
            (SymbolId::new(1), SlotIdx::new(1)),
        ],
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Secret)
}

#[test]
fn build_object_with_taint_joins_derived_from_secret() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;
    let (_obj, taint) = build_object_with_taint(
        &mut store,
        &run,
        &[
            (SymbolId::new(0), SlotIdx::new(0)),
            (SymbolId::new(1), SlotIdx::new(1)),
        ],
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::DerivedFromSecret)
}

// ===== build_list tests =====

#[test]
fn build_list_empty_items_creates_empty_list() -> Result<(), String> {
    let mut store = ValueStore::new();
    let run = test_frame(1)?;
    let list = build_list(&mut store, &run, &[]).map_err(|e| e.to_string())?;
    let items = store.list(list).map_err(|e| e.to_string())?;
    ensure_equal(items.is_empty(), true)
}

#[test]
fn build_list_single_item_reads_correct_slot() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
        .map_err(|e| e.to_string())?;
    let list = build_list(&mut store, &run, &[SlotIdx::new(0)]).map_err(|e| e.to_string())?;
    let items = store.list(list).map_err(|e| e.to_string())?;
    ensure_equal(items.len(), 1)?;
    ensure_equal(items[0], SlotValue::Bool(false))
}

#[test]
fn build_list_preserves_order() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(3)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(2))
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(2), SlotValue::I64(3))
        .map_err(|e| e.to_string())?;
    let list = build_list(
        &mut store,
        &run,
        &[SlotIdx::new(2), SlotIdx::new(0), SlotIdx::new(1)],
    )
    .map_err(|e| e.to_string())?;
    let items = store.list(list).map_err(|e| e.to_string())?;
    ensure_equal(items[0], SlotValue::I64(3))?;
    ensure_equal(items[1], SlotValue::I64(1))?;
    ensure_equal(items[2], SlotValue::I64(2))
}

#[test]
fn build_list_rejects_out_of_bounds_slot_without_inserting() -> Result<(), String> {
    let mut store = ValueStore::new();
    let run = test_frame(1)?;
    let result = build_list(&mut store, &run, &[SlotIdx::new(10)]);
    match result {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(10) => {
            ensure_equal(store.list_count(), 0)
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ===== build_list_with_taint tests =====

#[test]
fn build_list_with_taint_all_clean_produces_clean() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(2))
        .map_err(|e| e.to_string())?;
    let (_list, taint) =
        build_list_with_taint(&mut store, &run, &[SlotIdx::new(0), SlotIdx::new(1)])
            .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Clean)
}

#[test]
fn build_list_with_taint_joins_secret_from_one_slot() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret)
        .map_err(|e| e.to_string())?;
    let (_list, taint) =
        build_list_with_taint(&mut store, &run, &[SlotIdx::new(0), SlotIdx::new(1)])
            .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Secret)
}

#[test]
fn build_list_with_taint_joins_derived_from_secret() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Null, Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Clean)
        .map_err(|e| e.to_string())?;
    let (_list, taint) =
        build_list_with_taint(&mut store, &run, &[SlotIdx::new(0), SlotIdx::new(1)])
            .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::DerivedFromSecret)
}

#[test]
fn build_object_with_taint_rejects_out_of_bounds_slot() {
    let mut store = ValueStore::new();
    let run = test_frame(1).expect("frame");
    let result = build_object_with_taint(&mut store, &run, &[(SymbolId::new(0), SlotIdx::new(5))]);
    assert_eq!(
        result,
        Err(EngineError::SlotOutOfBounds {
            slot: SlotIdx::new(5),
        })
    );
    assert_eq!(store.object_count(), 0);
}

#[test]
fn build_list_with_taint_rejects_out_of_bounds_slot() {
    let mut store = ValueStore::new();
    let run = test_frame(1).expect("frame");
    let result = build_list_with_taint(&mut store, &run, &[SlotIdx::new(10)]);
    assert_eq!(
        result,
        Err(EngineError::SlotOutOfBounds {
            slot: SlotIdx::new(10),
        })
    );
    assert_eq!(store.list_count(), 0);
}

#[test]
fn build_object_with_taint_rejects_uninitialized_slot() {
    let mut store = ValueStore::new();
    let run = test_frame(2).expect("frame");
    // Slot 0 is uninitialized
    let result = build_object_with_taint(&mut store, &run, &[(SymbolId::new(0), SlotIdx::new(0))]);
    assert_eq!(
        result,
        Err(EngineError::SlotUninitialized {
            slot: SlotIdx::new(0),
        })
    );
}

#[test]
fn build_list_with_taint_rejects_uninitialized_slot() {
    let mut store = ValueStore::new();
    let run = test_frame(2).expect("frame");
    // Slot 0 is uninitialized
    let result = build_list_with_taint(&mut store, &run, &[SlotIdx::new(0)]);
    assert_eq!(
        result,
        Err(EngineError::SlotUninitialized {
            slot: SlotIdx::new(0),
        })
    );
}
