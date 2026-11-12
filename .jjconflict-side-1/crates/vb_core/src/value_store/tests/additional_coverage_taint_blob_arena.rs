// =========================================================================
// Additional coverage: Taint propagation paths
// =========================================================================

#[test]
fn value_store_list_with_explicit_taint_clean() -> Result<(), String> {
    let mut store = ValueStore::new();
    let values = vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice();
    let taints = vec![Taint::Clean, Taint::Clean].into_boxed_slice();
    let list_id = store
        .insert_list_with_taint(values, taints)
        .map_err(|e| e.to_string())?;
    let (_, taint0) = store
        .list_item_with_taint(list_id, 0)
        .map_err(|e| e.to_string())?;
    let (_, taint1) = store
        .list_item_with_taint(list_id, 1)
        .map_err(|e| e.to_string())?;
    assert_eq!(taint0, Taint::Clean);
    assert_eq!(taint1, Taint::Clean);
    Ok(())
}

#[test]
fn value_store_list_with_explicit_taint_secret() -> Result<(), String> {
    let mut store = ValueStore::new();
    let values = vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice();
    let taints = vec![Taint::Secret, Taint::DerivedFromSecret].into_boxed_slice();
    let list_id = store
        .insert_list_with_taint(values, taints)
        .map_err(|e| e.to_string())?;
    let (_, taint0) = store
        .list_item_with_taint(list_id, 0)
        .map_err(|e| e.to_string())?;
    let (_, taint1) = store
        .list_item_with_taint(list_id, 1)
        .map_err(|e| e.to_string())?;
    assert_eq!(taint0, Taint::Secret);
    assert_eq!(taint1, Taint::DerivedFromSecret);
    Ok(())
}

#[test]
fn value_store_object_with_explicit_taint() -> Result<(), String> {
    let mut store = ValueStore::new();
    let fields = vec![
        ObjectField::with_taint(SymbolId::new(1), SlotValue::I64(100), Taint::Secret),
        ObjectField::with_taint(
            SymbolId::new(2),
            SlotValue::I64(200),
            Taint::DerivedFromSecret,
        ),
    ]
    .into_boxed_slice();
    let obj_id = store.insert_object(fields).map_err(|e| e.to_string())?;
    let (val1, taint1) = store
        .object_field_with_taint(obj_id, SymbolId::new(1))
        .map_err(|e| e.to_string())?;
    let (val2, taint2) = store
        .object_field_with_taint(obj_id, SymbolId::new(2))
        .map_err(|e| e.to_string())?;
    assert_eq!(val1, SlotValue::I64(100));
    assert_eq!(taint1, Taint::Secret);
    assert_eq!(val2, SlotValue::I64(200));
    assert_eq!(taint2, Taint::DerivedFromSecret);
    Ok(())
}

// =========================================================================
// Additional coverage: list_taints and object_taint_index access paths
// =========================================================================

#[test]
fn value_store_list_item_with_taint_on_clean_list() -> Result<(), String> {
    let mut store = ValueStore::new();
    let list_id = store
        .insert_list(vec![SlotValue::Bool(true)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let (value, taint) = store
        .list_item_with_taint(list_id, 0)
        .map_err(|e| e.to_string())?;
    assert_eq!(value, SlotValue::Bool(true));
    assert_eq!(taint, Taint::Clean);
    Ok(())
}

#[test]
fn value_store_object_field_with_taint_on_clean_object() -> Result<(), String> {
    let mut store = ValueStore::new();
    let fields = vec![ObjectField::clean(SymbolId::new(5), SlotValue::I64(42))].into_boxed_slice();
    let obj_id = store.insert_object(fields).map_err(|e| e.to_string())?;
    let (value, taint) = store
        .object_field_with_taint(obj_id, SymbolId::new(5))
        .map_err(|e| e.to_string())?;
    assert_eq!(value, SlotValue::I64(42));
    assert_eq!(taint, Taint::Clean);
    Ok(())
}

#[test]
fn value_store_list_item_with_taint_index_out_of_bounds() -> Result<(), String> {
    let mut store = ValueStore::new();
    let list_id = store
        .insert_list(vec![SlotValue::Null].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let result = store.list_item_with_taint(list_id, u32::MAX);
    match result {
        Err(CoreError::ListIndexOutOfBounds { .. }) => Ok(()),
        other => Err(format!("expected ListIndexOutOfBounds, got {:?}", other)),
    }
}

#[test]
fn value_store_object_field_with_taint_object_not_found() -> Result<(), String> {
    let store = ValueStore::new();
    let result = store.object_field_with_taint(ObjectId::new(0), SymbolId::new(0));
    match result {
        Err(CoreError::ObjectOutOfBounds { .. }) => Ok(()),
        other => Err(format!("expected ObjectOutOfBounds, got {:?}", other)),
    }
}

#[test]
fn value_store_object_field_with_taint_field_not_found() -> Result<(), String> {
    let mut store = ValueStore::new();
    let fields = vec![ObjectField::clean(SymbolId::new(1), SlotValue::Null)].into_boxed_slice();
    let obj_id = store.insert_object(fields).map_err(|e| e.to_string())?;
    let result = store.object_field_with_taint(obj_id, SymbolId::new(99));
    match result {
        Err(CoreError::ObjectFieldNotFound { .. }) => Ok(()),
        other => Err(format!("expected ObjectFieldNotFound, got {:?}", other)),
    }
}

// =========================================================================
// Additional coverage: Blob operations
// =========================================================================

#[test]
fn value_store_blob_non_empty_data() -> Result<(), String> {
    let mut store = ValueStore::new();
    let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let blob_id = store
        .insert_blob(Bytes::from(data.clone()))
        .map_err(|e| e.to_string())?;
    let retrieved = store.blob(blob_id).map_err(|e| e.to_string())?;
    assert_eq!(retrieved, &data[..]);
    Ok(())
}

#[test]
fn value_store_blob_multiple_inserts() -> Result<(), String> {
    let mut store = ValueStore::new();
    let blob0 = store
        .insert_blob(Bytes::from_static(b"first"))
        .map_err(|e| e.to_string())?;
    let blob1 = store
        .insert_blob(Bytes::from_static(b"second"))
        .map_err(|e| e.to_string())?;
    let blob2 = store
        .insert_blob(Bytes::from_static(b"third"))
        .map_err(|e| e.to_string())?;
    assert_eq!(store.blob(blob0).map_err(|e| e.to_string())?, b"first");
    assert_eq!(store.blob(blob1).map_err(|e| e.to_string())?, b"second");
    assert_eq!(store.blob(blob2).map_err(|e| e.to_string())?, b"third");
    Ok(())
}

#[test]
fn value_store_blob_zero_byte_insert() -> Result<(), String> {
    let mut store = ValueStore::new();
    let data = vec![0u8; 100];
    let blob_id = store
        .insert_blob(Bytes::from(data.clone()))
        .map_err(|e| e.to_string())?;
    let retrieved = store.blob(blob_id).map_err(|e| e.to_string())?;
    assert_eq!(retrieved.len(), 100);
    assert!(retrieved.iter().all(|&b| b == 0));
    Ok(())
}

// =========================================================================
// Additional coverage: total_arena_count and max_arena_entries
// =========================================================================

#[test]
fn value_store_total_arena_count_empty() -> Result<(), String> {
    let store = ValueStore::new();
    assert_eq!(store.total_arena_count(), 0);
    Ok(())
}

#[test]
fn value_store_total_arena_count_with_entries() -> Result<(), String> {
    let mut store = ValueStore::new();
    store
        .insert_symbol(Box::<str>::from("sym"))
        .map_err(|e| e.to_string())?;
    store
        .insert_list(vec![SlotValue::Null].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    store
        .insert_object(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    store
        .insert_blob(Bytes::from_static(b"x"))
        .map_err(|e| e.to_string())?;
    assert_eq!(store.total_arena_count(), 4);
    Ok(())
}

#[test]
fn value_store_max_arena_entries_zero_uncapped() -> Result<(), String> {
    let store = ValueStore::new();
    assert_eq!(store.max_arena_entries(), 0);
    Ok(())
}

#[test]
fn value_store_max_arena_entries_from_with_max_slots() -> Result<(), String> {
    let store = ValueStore::with_max_slots(100);
    assert_eq!(store.max_arena_entries(), 100);
    Ok(())
}

#[test]
fn value_store_total_arena_count_saturating() -> Result<(), String> {
    let mut store = ValueStore::new();
    for _ in 0..1000 {
        store
            .insert_symbol(Box::<str>::from("x"))
            .map_err(|e| e.to_string())?;
    }
    assert_eq!(store.total_arena_count(), 1000);
    Ok(())
}
