#![forbid(unsafe_code)]

use super::{ObjectField, ValueStore};
use crate::errors::CoreError;
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::limits::{
    MAX_BLOB_BYTES_PER_VALUE, MAX_LIST_ITEMS_PER_VALUE, MAX_SYMBOL_BYTES_PER_VALUE,
};
use crate::value::{ConstValue, FiniteF64, SlotValue, Taint};
use bytes::Bytes;

// =============================================================================
// 1. Symbol Interning
// =============================================================================

#[test]
fn symbol_intern_stores_and_retrieves_string() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store
        .insert_symbol(Box::<str>::from("hello_world"))
        .map_err(|e| e.to_string())?;
    let retrieved = store.symbol(id).map_err(|e| e.to_string())?;
    if retrieved != "hello_world" {
        return Err(format!("expected 'hello_world', got '{retrieved}'"));
    }
    Ok(())
}

#[test]
fn symbol_intern_sequential_ids_are_monotonic() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id0 = store
        .insert_symbol(Box::<str>::from("first"))
        .map_err(|e| e.to_string())?;
    let id1 = store
        .insert_symbol(Box::<str>::from("second"))
        .map_err(|e| e.to_string())?;
    let id2 = store
        .insert_symbol(Box::<str>::from("third"))
        .map_err(|e| e.to_string())?;
    if id0.get() != 0 || id1.get() != 1 || id2.get() != 2 {
        return Err(format!(
            "expected monotonic ids 0,1,2 got {},{},{}",
            id0.get(),
            id1.get(),
            id2.get()
        ));
    }
    Ok(())
}

#[test]
fn symbol_intern_empty_string_is_valid() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store
        .insert_symbol(Box::<str>::from(""))
        .map_err(|e| e.to_string())?;
    let retrieved = store.symbol(id).map_err(|e| e.to_string())?;
    if !retrieved.is_empty() {
        return Err("empty string must be empty".into());
    }
    Ok(())
}

#[test]
fn symbol_intern_same_string_distinct_inserts_get_distinct_ids() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id0 = store
        .insert_symbol(Box::<str>::from("dup"))
        .map_err(|e| e.to_string())?;
    let id1 = store
        .insert_symbol(Box::<str>::from("dup"))
        .map_err(|e| e.to_string())?;
    if id0 == id1 {
        return Err("two inserts of same string must yield distinct ids".into());
    }
    if store.symbol(id0).map_err(|e| e.to_string())? != "dup" {
        return Err("id0 payload mismatch".into());
    }
    if store.symbol(id1).map_err(|e| e.to_string())? != "dup" {
        return Err("id1 payload mismatch".into());
    }
    if store.symbol_count() != 2 {
        return Err(format!(
            "expected symbol_count=2, got {}",
            store.symbol_count()
        ));
    }
    Ok(())
}

#[test]
fn symbol_intern_rejects_over_max_length() -> Result<(), String> {
    let mut store = ValueStore::new();
    let too_long = "x".repeat(MAX_SYMBOL_BYTES_PER_VALUE.saturating_add(1));
    store.insert_symbol(too_long.into_boxed_str()) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "symbol_bytes",
        }) => Ok(()),
        other => Err(format!("expected symbol_bytes limit, got {other:?}")),
    }
}

// =============================================================================
// 2. List Operations
// =============================================================================

#[test]
fn list_create_and_iterate_all_elements() -> Result<(), String> {
    let mut store = ValueStore::new();
    let values = vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice();
    let id = store.insert_list(values).map_err(|e| e.to_string())?;
    let items = store.list(id).map_err(|e| e.to_string())?;
    if items.len() != 3 {
        return Err(format!("expected length 3, got {}", items.len()));
    }
    if items[0] != SlotValue::I64(1) {
        return Err("index 0 mismatch".into());
    }
    if items[1] != SlotValue::I64(2) {
        return Err("index 1 mismatch".into());
    }
    if items[2] != SlotValue::I64(3) {
        return Err("index 2 mismatch".into());
    }
    Ok(())
}

#[test]
fn list_single_element_create_and_access() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store
        .insert_list(vec![SlotValue::Bool(true)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let item = store.list_item(id, 0).map_err(|e| e.to_string())?;
    if item != SlotValue::Bool(true) {
        return Err(format!("expected Bool(true), got {item:?}"));
    }
    Ok(())
}

#[test]
fn list_empty_create_and_iterate() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let items = store.list(id).map_err(|e| e.to_string())?;
    if !items.is_empty() {
        return Err("empty list must yield empty slice".into());
    }
    Ok(())
}

#[test]
fn list_nested_resolves_correctly() -> Result<(), String> {
    let mut store = ValueStore::new();
    let inner_id = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let outer_id = store
        .insert_list(vec![SlotValue::List(inner_id)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let outer_items = store.list(outer_id).map_err(|e| e.to_string())?;
    if outer_items.len() != 1 {
        return Err(format!("expected 1 outer item, got {}", outer_items.len()));
    }
    let resolved = store.list(inner_id).map_err(|e| e.to_string())?;
    if resolved[0] != SlotValue::I64(1) {
        return Err("nested list item mismatch".into());
    }
    Ok(())
}

#[test]
fn list_with_numeric_range_elements() -> Result<(), String> {
    let mut store = ValueStore::new();
    let values: Vec<SlotValue> = (0..10).map(|i| SlotValue::I64(i)).collect();
    let id = store
        .insert_list(values.into_boxed_slice())
        .map_err(|e| e.to_string())?;
    for i in 0..10u32 {
        let item = store.list_item(id, i).map_err(|e| e.to_string())?;
        if item != SlotValue::I64(i64::from(i)) {
            return Err(format!("item at index {i} mismatch"));
        }
    }
    Ok(())
}

#[test]
fn list_rejects_payload_over_hard_bound() -> Result<(), String> {
    let mut store = ValueStore::new();
    let values =
        vec![SlotValue::Null; MAX_LIST_ITEMS_PER_VALUE.saturating_add(1)].into_boxed_slice();
    match store.insert_list(values) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "list_items",
        }) => Ok(()),
        other => Err(format!("expected list_items limit, got {other:?}")),
    }
}

// =============================================================================
// 3. Object Operations
// =============================================================================

#[test]
fn object_field_lookup_by_key_resolves_value() -> Result<(), String> {
    let mut store = ValueStore::new();
    let key_a = SymbolId::new(10);
    let key_b = SymbolId::new(20);
    let obj_id = store
        .insert_object(
            vec![
                ObjectField::clean(key_a, SlotValue::I64(100)),
                ObjectField::clean(key_b, SlotValue::I64(200)),
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    if store
        .object_field(obj_id, key_a)
        .map_err(|e| e.to_string())?
        != SlotValue::I64(100)
    {
        return Err("key_a must resolve to 100".into());
    }
    if store
        .object_field(obj_id, key_b)
        .map_err(|e| e.to_string())?
        != SlotValue::I64(200)
    {
        return Err("key_b must resolve to 200".into());
    }
    Ok(())
}

#[test]
fn object_iteration_respects_insertion_order() -> Result<(), String> {
    let mut store = ValueStore::new();
    let obj_id = store
        .insert_object(
            vec![
                ObjectField::clean(SymbolId::new(1), SlotValue::I64(1)),
                ObjectField::clean(SymbolId::new(2), SlotValue::I64(2)),
                ObjectField::clean(SymbolId::new(3), SlotValue::I64(3)),
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let fields = store.object(obj_id).map_err(|e| e.to_string())?;
    if fields.len() != 3 {
        return Err(format!("expected 3 fields, got {}", fields.len()));
    }
    if fields[0].value != SlotValue::I64(1)
        || fields[1].value != SlotValue::I64(2)
        || fields[2].value != SlotValue::I64(3)
    {
        return Err("field values out of order".into());
    }
    Ok(())
}

#[test]
fn object_cross_reference_multiple_objects_dont_leak_fields() -> Result<(), String> {
    let mut store = ValueStore::new();
    let key_only_first = SymbolId::new(1);
    let _obj0 = store
        .insert_object(
            vec![ObjectField::clean(key_only_first, SlotValue::Bool(true))].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let obj1 = store
        .insert_object(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    match store.object_field(obj1, key_only_first) {
        Err(CoreError::ObjectFieldNotFound { field }) => {
            if field != key_only_first {
                return Err(format!("expected field {key_only_first:?}, got {field:?}"));
            }
            Ok(())
        }
        other => Err(format!("expected ObjectFieldNotFound, got {other:?}")),
    }
}

#[test]
fn object_empty_object_is_valid() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store
        .insert_object(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let fields = store.object(id).map_err(|e| e.to_string())?;
    if !fields.is_empty() {
        return Err("empty object must yield empty fields".into());
    }
    Ok(())
}

#[test]
fn object_duplicate_key_first_wins_for_query() -> Result<(), String> {
    let mut store = ValueStore::new();
    let dup_key = SymbolId::new(42);
    let obj_id = store
        .insert_object(
            vec![
                ObjectField::clean(dup_key, SlotValue::I64(1)),
                ObjectField::clean(dup_key, SlotValue::I64(2)),
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    if store
        .object_field(obj_id, dup_key)
        .map_err(|e| e.to_string())?
        != SlotValue::I64(1)
    {
        return Err("duplicate key must resolve to first value".into());
    }
    Ok(())
}

#[test]
fn object_missing_field_returns_object_field_not_found() -> Result<(), String> {
    let mut store = ValueStore::new();
    let obj_id = store
        .insert_object(
            vec![ObjectField::clean(SymbolId::new(5), SlotValue::Null)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let missing_key = SymbolId::new(99);
    match store.object_field(obj_id, missing_key) {
        Err(CoreError::ObjectFieldNotFound { field }) => {
            if field != missing_key {
                return Err(format!("expected field {missing_key:?}, got {field:?}"));
            }
            Ok(())
        }
        other => Err(format!("expected ObjectFieldNotFound, got {other:?}")),
    }
}

// =============================================================================
// 4. Blob Storage
// =============================================================================

#[test]
fn blob_stores_and_retrieves_binary_data() -> Result<(), String> {
    let mut store = ValueStore::new();
    let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let id = store
        .insert_blob(Bytes::from(data.clone()))
        .map_err(|e| e.to_string())?;
    let retrieved = store.blob(id).map_err(|e| e.to_string())?;
    if retrieved != &data[..] {
        return Err("blob data must round-trip exactly".into());
    }
    Ok(())
}

#[test]
fn blob_empty_blob_is_valid() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store.insert_blob(Bytes::new()).map_err(|e| e.to_string())?;
    let retrieved = store.blob(id).map_err(|e| e.to_string())?;
    if !retrieved.is_empty() {
        return Err("empty blob must be empty".into());
    }
    Ok(())
}

#[test]
fn blob_max_size_blob_accepted() -> Result<(), String> {
    let mut store = ValueStore::new();
    let data = vec![0xAB_u8; MAX_BLOB_BYTES_PER_VALUE];
    let id = store
        .insert_blob(Bytes::from(data))
        .map_err(|e| e.to_string())?;
    let retrieved = store.blob(id).map_err(|e| e.to_string())?;
    if retrieved.len() != MAX_BLOB_BYTES_PER_VALUE {
        return Err(format!(
            "expected {} bytes, got {}",
            MAX_BLOB_BYTES_PER_VALUE,
            retrieved.len()
        ));
    }
    Ok(())
}

#[test]
fn blob_multiple_inserts_preserve_independence() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id0 = store
        .insert_blob(Bytes::from_static(b"first"))
        .map_err(|e| e.to_string())?;
    let id1 = store
        .insert_blob(Bytes::from_static(b"second"))
        .map_err(|e| e.to_string())?;
    let id2 = store
        .insert_blob(Bytes::from_static(b"third"))
        .map_err(|e| e.to_string())?;
    if store.blob(id0).map_err(|e| e.to_string())? != b"first" {
        return Err("blob 0 must be 'first'".into());
    }
    if store.blob(id1).map_err(|e| e.to_string())? != b"second" {
        return Err("blob 1 must be 'second'".into());
    }
    if store.blob(id2).map_err(|e| e.to_string())? != b"third" {
        return Err("blob 2 must be 'third'".into());
    }
    Ok(())
}

#[test]
fn blob_rejects_over_max_size() -> Result<(), String> {
    let mut store = ValueStore::new();
    let data = vec![0u8; MAX_BLOB_BYTES_PER_VALUE.saturating_add(1)];
    match store.insert_blob(Bytes::from(data)) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "blob_bytes",
        }) => Ok(()),
        other => Err(format!("expected blob_bytes limit, got {other:?}")),
    }
}

// =============================================================================
// 5. Handle-Based Access
// =============================================================================

#[test]
fn handle_valid_after_insert_symbol() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store
        .insert_symbol(Box::<str>::from("valid"))
        .map_err(|e| e.to_string())?;
    let result = store.symbol(id);
    if result.is_err() {
        return Err(format!("valid handle must succeed: {result:?}"));
    }
    Ok(())
}

#[test]
fn handle_valid_after_insert_list() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store
        .insert_list(vec![SlotValue::Null].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let result = store.list(id);
    if result.is_err() {
        return Err(format!("valid list handle must succeed: {result:?}"));
    }
    Ok(())
}

#[test]
fn handle_valid_after_insert_object() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store
        .insert_object(
            vec![ObjectField::clean(SymbolId::new(0), SlotValue::Null)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let result = store.object(id);
    if result.is_err() {
        return Err(format!("valid object handle must succeed: {result:?}"));
    }
    Ok(())
}

#[test]
fn handle_valid_after_insert_blob() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store
        .insert_blob(Bytes::from_static(b"data"))
        .map_err(|e| e.to_string())?;
    let result = store.blob(id);
    if result.is_err() {
        return Err(format!("valid blob handle must succeed: {result:?}"));
    }
    Ok(())
}

#[test]
fn handle_invalid_symbol_on_empty_store_returns_error() {
    let store = ValueStore::new();
    let result = store.symbol(SymbolId::new(0));
    assert_eq!(
        result,
        Err(CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(0),
        })
    );
}

#[test]
fn handle_forged_max_symbol_id_returns_error_not_panic() {
    let store = ValueStore::new();
    let result = store.symbol(SymbolId::new(u32::MAX));
    assert_eq!(
        result,
        Err(CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(u32::MAX),
        })
    );
}

#[test]
fn handle_forged_max_blob_id_returns_error_not_panic() {
    let store = ValueStore::new();
    let result = store.blob(BlobId::new(u64::MAX));
    assert_eq!(
        result,
        Err(CoreError::BlobOutOfBounds {
            blob: BlobId::new(u64::MAX),
        })
    );
}

#[test]
fn handle_type_safety_cross_arena_index_does_not_confuse_types() -> Result<(), String> {
    let mut store = ValueStore::new();
    let sym_id = store
        .insert_symbol(Box::<str>::from("only_symbol"))
        .map_err(|e| e.to_string())?;
    let list_id = store
        .insert_list(vec![SlotValue::Bool(true)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    if sym_id.get() != 0 || list_id.get() != 0 {
        return Err("both must be at index 0 for this test".into());
    }
    if store.symbol(sym_id).map_err(|e| e.to_string())? != "only_symbol" {
        return Err("symbol must resolve via SymbolId".into());
    }
    if store.list_item(list_id, 0).map_err(|e| e.to_string())? != SlotValue::Bool(true) {
        return Err("list must resolve via ListId".into());
    }
    Ok(())
}

#[test]
fn handle_list_item_out_of_bounds_index_rejected() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    match store.list_item(id, 1) {
        Err(CoreError::ListIndexOutOfBounds { index: 1 }) => Ok(()),
        other => Err(format!("expected ListIndexOutOfBounds, got {other:?}")),
    }
}

#[test]
fn handle_object_field_on_forged_object_id_returns_error() {
    let store = ValueStore::new();
    let result = store.object_field(ObjectId::new(u32::MAX), SymbolId::new(0));
    assert_eq!(
        result,
        Err(CoreError::ObjectOutOfBounds {
            object: ObjectId::new(u32::MAX),
        })
    );
}

// =============================================================================
// 6. Value Type Correctness (Store-Integrated)
// =============================================================================

#[test]
fn value_store_accepts_all_primitive_slot_values_in_list() -> Result<(), String> {
    let mut store = ValueStore::new();
    let finite = FiniteF64::new(3.14).map_err(|e| e.to_string())?;
    let values = vec![
        SlotValue::Null,
        SlotValue::Bool(true),
        SlotValue::Bool(false),
        SlotValue::I64(0),
        SlotValue::I64(-1),
        SlotValue::I64(i64::MAX),
        SlotValue::I64(i64::MIN),
        SlotValue::F64(finite),
    ]
    .into_boxed_slice();
    let id = store.insert_list(values).map_err(|e| e.to_string())?;
    let items = store.list(id).map_err(|e| e.to_string())?;
    if items.len() != 8 {
        return Err(format!("expected 8 items, got {}", items.len()));
    }
    if items[0] != SlotValue::Null {
        return Err("item 0 mismatch".into());
    }
    if items[1] != SlotValue::Bool(true) {
        return Err("item 1 mismatch".into());
    }
    if items[2] != SlotValue::Bool(false) {
        return Err("item 2 mismatch".into());
    }
    if items[3] != SlotValue::I64(0) {
        return Err("item 3 mismatch".into());
    }
    if items[4] != SlotValue::I64(-1) {
        return Err("item 4 mismatch".into());
    }
    if items[5] != SlotValue::I64(i64::MAX) {
        return Err("item 5 mismatch".into());
    }
    if items[6] != SlotValue::I64(i64::MIN) {
        return Err("item 6 mismatch".into());
    }
    if items[7] != SlotValue::F64(finite) {
        return Err("item 7 mismatch".into());
    }
    Ok(())
}

#[test]
fn value_null_is_not_bool_true_in_store_context() {
    assert!(!SlotValue::Null.is_true());
}

#[test]
fn value_i64_and_f64_both_report_number_type() -> Result<(), String> {
    let finite = FiniteF64::new(1.0).map_err(|e| e.to_string())?;
    if SlotValue::I64(0).type_name() != "number" {
        return Err("I64 must report 'number'".into());
    }
    if SlotValue::F64(finite).type_name() != "number" {
        return Err("F64 must report 'number'".into());
    }
    Ok(())
}

#[test]
fn value_const_to_slot_all_variants_roundtrip_in_store() -> Result<(), String> {
    let finite = FiniteF64::new(2.0).map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();
    {
        let cv = ConstValue::Null;
        let sv = cv.to_slot_value().map_err(|e| e.to_string())?;
        if sv != SlotValue::Null {
            return Err("ConstValue::Null mismatch".into());
        }
    }
    {
        let cv = ConstValue::Bool(true);
        let sv = cv.to_slot_value().map_err(|e| e.to_string())?;
        if sv != SlotValue::Bool(true) {
            return Err("ConstValue::Bool(true) mismatch".into());
        }
    }
    {
        let cv = ConstValue::I64(-42);
        let sv = cv.to_slot_value().map_err(|e| e.to_string())?;
        if sv != SlotValue::I64(-42) {
            return Err("ConstValue::I64(-42) mismatch".into());
        }
    }
    {
        let cv = ConstValue::F64(finite);
        let sv = cv.to_slot_value().map_err(|e| e.to_string())?;
        if sv != SlotValue::F64(finite) {
            return Err("ConstValue::F64 mismatch".into());
        }
    }
    {
        let sym_id = store
            .insert_symbol(Box::<str>::from("test_sym"))
            .map_err(|e| e.to_string())?;
        let cv = ConstValue::Symbol(sym_id);
        let sv = cv.to_slot_value().map_err(|e| e.to_string())?;
        if sv != SlotValue::Symbol(sym_id) {
            return Err("ConstValue::Symbol mismatch".into());
        }
    }
    Ok(())
}

// =============================================================================
// 7. Value Equality / Comparison
// =============================================================================

#[test]
fn slot_value_equality_null_distinct_from_bool_false() {
    assert_ne!(SlotValue::Null, SlotValue::Bool(false));
}

#[test]
fn slot_value_equality_i64_vs_f64_same_numeric_value_are_distinct() -> Result<(), String> {
    let finite = FiniteF64::new(0.0).map_err(|e| e.to_string())?;
    assert_ne!(SlotValue::I64(0), SlotValue::F64(finite));
    Ok(())
}

#[test]
fn slot_value_equality_different_handle_types_different() {
    assert_ne!(
        SlotValue::Symbol(SymbolId::new(1)),
        SlotValue::List(ListId::new(1))
    );
    assert_ne!(
        SlotValue::List(ListId::new(1)),
        SlotValue::Object(ObjectId::new(1))
    );
    assert_ne!(
        SlotValue::Object(ObjectId::new(1)),
        SlotValue::Blob(BlobId::new(1))
    );
}

#[test]
fn slot_value_clone_preserves_equality_when_stored_in_list() -> Result<(), String> {
    let mut store = ValueStore::new();
    let original = SlotValue::I64(99);
    let cloned = original;
    if original != cloned {
        return Err("clone must preserve equality".into());
    }
    let id = store
        .insert_list(vec![original].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let retrieved = store.list_item(id, 0).map_err(|e| e.to_string())?;
    if retrieved != cloned {
        return Err("stored value must match cloned value".into());
    }
    Ok(())
}

// =============================================================================
// 8. Store Integrity: Clone, Counts, Capacities
// =============================================================================

#[test]
fn store_clone_produces_equal_store() -> Result<(), String> {
    let mut store = ValueStore::new();
    store
        .insert_symbol(Box::<str>::from("alpha"))
        .map_err(|e| e.to_string())?;
    store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let cloned = store.clone();
    if store != cloned {
        return Err("clone must be equal".into());
    }
    if store.symbol_count() != cloned.symbol_count() {
        return Err("symbol count must match".into());
    }
    if store.list_count() != cloned.list_count() {
        return Err("list count must match".into());
    }
    Ok(())
}

#[test]
fn store_default_equals_new() {
    let default: ValueStore = Default::default();
    let constructed = ValueStore::new();
    assert_eq!(default, constructed);
}

#[test]
fn store_counts_accurate_after_mixed_inserts() -> Result<(), String> {
    let mut store = ValueStore::new();
    if store.symbol_count() != 0
        || store.list_count() != 0
        || store.object_count() != 0
        || store.blob_count() != 0
    {
        return Err("initial counts must be zero".into());
    }
    store
        .insert_symbol(Box::<str>::from("a"))
        .map_err(|e| e.to_string())?;
    store
        .insert_symbol(Box::<str>::from("b"))
        .map_err(|e| e.to_string())?;
    if store.symbol_count() != 2 {
        return Err(format!(
            "expected symbol_count=2, got {}",
            store.symbol_count()
        ));
    }
    store
        .insert_list(vec![SlotValue::Null].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    if store.list_count() != 1 {
        return Err(format!("expected list_count=1, got {}", store.list_count()));
    }
    store
        .insert_object(
            vec![ObjectField::clean(SymbolId::new(0), SlotValue::Null)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    store
        .insert_object(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    if store.object_count() != 2 {
        return Err(format!(
            "expected object_count=2, got {}",
            store.object_count()
        ));
    }
    store
        .insert_blob(Bytes::from_static(b"x"))
        .map_err(|e| e.to_string())?;
    if store.blob_count() != 1 {
        return Err(format!("expected blob_count=1, got {}", store.blob_count()));
    }
    Ok(())
}

#[test]
fn store_total_arena_count_sums_all_arenas() -> Result<(), String> {
    let mut store = ValueStore::new();
    if store.total_arena_count() != 0 {
        return Err("empty store arena count must be 0".into());
    }
    store
        .insert_symbol(Box::<str>::from("s"))
        .map_err(|e| e.to_string())?;
    if store.total_arena_count() != 1 {
        return Err("arena count must be 1 after 1 insert".into());
    }
    store
        .insert_list(vec![SlotValue::Null].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    store
        .insert_object(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    store
        .insert_blob(Bytes::from_static(b"b"))
        .map_err(|e| e.to_string())?;
    if store.total_arena_count() != 4 {
        return Err(format!(
            "arena count must be 4, got {}",
            store.total_arena_count()
        ));
    }
    Ok(())
}

#[test]
fn store_new_has_no_cap_and_allows_unlimited_inserts() -> Result<(), String> {
    let mut store = ValueStore::new();
    if store.max_arena_entries() != 0 {
        return Err("new store must have cap 0".into());
    }
    for _ in 0..100 {
        store
            .insert_symbol(Box::<str>::from("x"))
            .map_err(|e| e.to_string())?;
    }
    if store.total_arena_count() != 100 {
        return Err(format!("expected 100, got {}", store.total_arena_count()));
    }
    Ok(())
}

#[test]
fn store_with_max_slots_enforces_capacity() -> Result<(), String> {
    let mut store = ValueStore::with_max_slots(2);
    if store.max_arena_entries() != 2 {
        return Err("expected cap of 2".into());
    }
    store
        .insert_symbol(Box::<str>::from("a"))
        .map_err(|e| e.to_string())?;
    store
        .insert_list(vec![SlotValue::Null].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    if store.total_arena_count() != 2 {
        return Err("arena count must be exactly 2".into());
    }
    match store.insert_symbol(Box::<str>::from("b")) {
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 2,
        }) => Ok(()),
        other => Err(format!("expected BudgetExceeded, got {other:?}")),
    }
}

#[test]
fn store_capacity_rejected_insert_does_not_mutate() -> Result<(), String> {
    let mut store = ValueStore::with_max_slots(1);
    let id = store
        .insert_symbol(Box::<str>::from("only"))
        .map_err(|e| e.to_string())?;
    let _ = store.insert_symbol(Box::<str>::from("overflow"));
    if store.symbol_count() != 1 {
        return Err("rejected insert must not change count".into());
    }
    if store.symbol(id).map_err(|e| e.to_string())? != "only" {
        return Err("rejected insert must not corrupt existing data".into());
    }
    Ok(())
}

// =============================================================================
// 9. Serialization Roundtrip (Postcard)
// =============================================================================

#[test]
fn postcard_roundtrip_slot_value_null() -> Result<(), String> {
    let val = SlotValue::Null;
    let bytes = postcard::to_allocvec(&val).map_err(|e| e.to_string())?;
    let recovered: SlotValue = postcard::from_bytes(&bytes).map_err(|e| e.to_string())?;
    if recovered != val {
        return Err("postcard roundtrip failed for Null".into());
    }
    Ok(())
}

#[test]
fn postcard_roundtrip_slot_value_i64_boundaries() -> Result<(), String> {
    for v in [i64::MIN, -1_i64, 0_i64, 1_i64, i64::MAX] {
        let val = SlotValue::I64(v);
        let bytes = postcard::to_allocvec(&val).map_err(|e| e.to_string())?;
        let recovered: SlotValue = postcard::from_bytes(&bytes).map_err(|e| e.to_string())?;
        if recovered != val {
            return Err(format!("postcard roundtrip failed for I64({v})"));
        }
    }
    Ok(())
}

#[test]
fn postcard_roundtrip_slot_value_bool_both() -> Result<(), String> {
    for b in [true, false] {
        let val = SlotValue::Bool(b);
        let bytes = postcard::to_allocvec(&val).map_err(|e| e.to_string())?;
        let recovered: SlotValue = postcard::from_bytes(&bytes).map_err(|e| e.to_string())?;
        if recovered != val {
            return Err(format!("postcard roundtrip failed for Bool({b})"));
        }
    }
    Ok(())
}

#[test]
fn postcard_roundtrip_slot_value_handle_types() -> Result<(), String> {
    let vals = [
        SlotValue::Symbol(SymbolId::new(0)),
        SlotValue::Symbol(SymbolId::new(u32::MAX)),
        SlotValue::List(ListId::new(0)),
        SlotValue::List(ListId::new(u32::MAX)),
        SlotValue::Object(ObjectId::new(0)),
        SlotValue::Object(ObjectId::new(u32::MAX)),
        SlotValue::Blob(BlobId::new(0)),
        SlotValue::Blob(BlobId::new(u64::MAX)),
    ];
    for val in vals {
        let bytes = postcard::to_allocvec(&val).map_err(|e| e.to_string())?;
        let recovered: SlotValue = postcard::from_bytes(&bytes).map_err(|e| e.to_string())?;
        if recovered != val {
            return Err(format!("postcard roundtrip failed for {val:?}"));
        }
    }
    Ok(())
}

#[test]
fn postcard_roundtrip_finite_f64() -> Result<(), String> {
    let finite = FiniteF64::new(42.5).map_err(|e| e.to_string())?;
    let val = SlotValue::F64(finite);
    let bytes = postcard::to_allocvec(&val).map_err(|e| e.to_string())?;
    let recovered: SlotValue = postcard::from_bytes(&bytes).map_err(|e| e.to_string())?;
    if recovered != val {
        return Err("postcard roundtrip failed for F64".into());
    }
    Ok(())
}

#[test]
fn postcard_roundtrip_taint_all_variants() -> Result<(), String> {
    let variants = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Secret,
        Taint::Secret,
    ];
    for taint in variants {
        let bytes = postcard::to_allocvec(&taint).map_err(|e| e.to_string())?;
        let recovered: Taint = postcard::from_bytes(&bytes).map_err(|e| e.to_string())?;
        if recovered != taint {
            return Err(format!("postcard roundtrip failed for {taint:?}"));
        }
    }
    Ok(())
}

// =============================================================================
// 10. Taint Propagation Through Stores
// =============================================================================

#[test]
fn taint_list_insert_clean_default_taint_is_clean() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = store
        .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let (_, taint0) = store
        .list_item_with_taint(id, 0)
        .map_err(|e| e.to_string())?;
    let (_, taint1) = store
        .list_item_with_taint(id, 1)
        .map_err(|e| e.to_string())?;
    if taint0 != Taint::Clean || taint1 != Taint::Clean {
        return Err("default taint must be Clean".into());
    }
    Ok(())
}

#[test]
fn taint_list_insert_with_explicit_taint_preserves_values() -> Result<(), String> {
    let mut store = ValueStore::new();
    let values = vec![SlotValue::I64(10), SlotValue::I64(20)].into_boxed_slice();
    let taints = vec![Taint::Secret, Taint::DerivedFromSecret].into_boxed_slice();
    let id = store
        .insert_list_with_taint(values, taints)
        .map_err(|e| e.to_string())?;
    let (v0, t0) = store
        .list_item_with_taint(id, 0)
        .map_err(|e| e.to_string())?;
    let (v1, t1) = store
        .list_item_with_taint(id, 1)
        .map_err(|e| e.to_string())?;
    if v0 != SlotValue::I64(10) || t0 != Taint::Secret {
        return Err("taint item 0 mismatch".into());
    }
    if v1 != SlotValue::I64(20) || t1 != Taint::DerivedFromSecret {
        return Err("taint item 1 mismatch".into());
    }
    Ok(())
}

#[test]
fn taint_list_insert_taint_length_mismatch_rejected_and_not_mutate() -> Result<(), String> {
    let mut store = ValueStore::new();
    let baseline = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let values = vec![SlotValue::I64(10), SlotValue::I64(20)].into_boxed_slice();
    let taints = vec![Taint::Clean].into_boxed_slice();
    match store.insert_list_with_taint(values, taints) {
        Err(CoreError::InternalInvariantViolation { .. }) => {}
        other => return Err(format!("expected length mismatch, got {other:?}")),
    }
    if store.list_count() != 1 {
        return Err("failed taint insert must not change count".into());
    }
    if store.list_item(baseline, 0).map_err(|e| e.to_string())? != SlotValue::I64(1) {
        return Err("failed taint insert must not corrupt".into());
    }
    Ok(())
}

#[test]
fn taint_object_field_with_explicit_taint_preserves_values() -> Result<(), String> {
    let mut store = ValueStore::new();
    let key = SymbolId::new(1);
    let obj_id = store
        .insert_object(
            vec![ObjectField::with_taint(
                key,
                SlotValue::I64(42),
                Taint::Secret,
            )]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let (val, taint) = store
        .object_field_with_taint(obj_id, key)
        .map_err(|e| e.to_string())?;
    if val != SlotValue::I64(42) || taint != Taint::Secret {
        return Err("object taint mismatch".into());
    }
    Ok(())
}

#[test]
fn taint_object_duplicate_key_first_wins_for_taint() -> Result<(), String> {
    let mut store = ValueStore::new();
    let key = SymbolId::new(5);
    let obj_id = store
        .insert_object(
            vec![
                ObjectField::with_taint(key, SlotValue::I64(1), Taint::Secret),
                ObjectField::with_taint(key, SlotValue::I64(2), Taint::Clean),
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let (val, taint) = store
        .object_field_with_taint(obj_id, key)
        .map_err(|e| e.to_string())?;
    if val != SlotValue::I64(1) || taint != Taint::Secret {
        return Err("duplicate key first-wins must apply to taint".into());
    }
    Ok(())
}

// =============================================================================
// 11. Kani Verification Harnesses
// =============================================================================

#[cfg(kani)]
mod kani {
    use super::*;

    #[kani::proof]
    fn symbol_handle_valid_after_insert() {
        let mut store = ValueStore::new();
        let raw_input: &str = kani::any();
        let bounded = if raw_input.len() > 16 {
            &raw_input[..16]
        } else {
            raw_input
        };
        let len = bounded.len();
        kani::assume(len <= 16);
        match store.insert_symbol(Box::<str>::from(bounded)) {
            Ok(id) => match store.symbol(id) {
                Ok(resolved) => {
                    assert_eq!(resolved, bounded);
                }
                Err(_) => {
                    assert!(false, "valid handle must resolve");
                }
            },
            Err(_) => {}
        }
    }

    #[kani::proof]
    fn invalid_blob_handle_never_panics() {
        let store = ValueStore::new();
        let forged_id: u64 = kani::any();
        let blob_id = BlobId::new(forged_id);
        let _ = store.blob(blob_id);
    }

    #[kani::proof]
    fn value_store_cap_enforcement_rejects_over_cap() {
        let cap: u16 = kani::any();
        kani::assume(cap > 0 && cap <= 8);
        let mut store = ValueStore::with_max_slots(cap);
        let cap_u64 = u64::from(cap);
        let mut c: u16 = 0;
        while c < cap {
            let _ = store.insert_symbol(Box::<str>::from("x"));
            c = c.saturating_add(1);
        }
        if store.total_arena_count() != cap_u64 {
            return;
        }
        let result = store.insert_symbol(Box::<str>::from("overflow"));
        match result {
            Ok(_) => {
                assert!(false, "insert over cap must not succeed");
            }
            Err(CoreError::BudgetExceeded { budget, limit }) => {
                assert_eq!(budget, "max_slots");
                assert_eq!(limit, cap_u64);
            }
            Err(_) => {}
        }
    }
}

// =============================================================================
// 12. Proptest Properties
// =============================================================================

proptest::proptest! {
    #[test]
    fn proptest_symbol_insert_then_lookup_matches(
        s in "[a-zA-Z0-9_]{1,32}"
    ) {
        use proptest::prop_assert_eq;
        let mut store = ValueStore::new();
        let id = match store.insert_symbol(s.clone().into_boxed_str()).unwrap();
        let retrieved = store.symbol(id).unwrap();
        prop_assert_eq!(retrieved, s.as_str());
    }

    #[test]
    fn proptest_blob_roundtrip(
        data in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..1024)
    ) {
        use proptest::{prop_assert, prop_assert_eq};
        let mut store = ValueStore::new();
        let bytes = Bytes::from(data.clone());
        let id = store.insert_blob(bytes).unwrap();
        let retrieved = store.blob(id).unwrap();
        prop_assert_eq!(retrieved.len(), data.len());
        prop_assert!(retrieved == data.as_slice());
    }

    #[test]
    fn proptest_list_insert_then_length_matches(
        items in proptest::collection::vec(proptest::prelude::any::<i64>(), 0..32)
    ) {
        use proptest::prop_assert_eq;
        let mut store = ValueStore::new();
        let values: Vec<SlotValue> = items.iter().map(|i| SlotValue::I64(*i)).collect();
        let boxed_values: Box<[SlotValue]> = values.clone().into_boxed_slice();
        let id = store.insert_list(boxed_values).unwrap();
        let retrieved = store.list(id).unwrap();
        prop_assert_eq!(retrieved.len(), values.len());
        for (i, expected) in values.iter().enumerate() {
            prop_assert_eq!(retrieved[i], *expected);
        }
    }
}
