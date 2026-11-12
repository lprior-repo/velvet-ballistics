// =========================================================================
// Phase 2 adversarial BDD tests — value store overflow & security
// =========================================================================

// --- Blob at one byte over the limit is rejected ---

#[test]
fn value_store_blob_one_byte_over_limit_is_rejected() -> Result<(), String> {
    let mut store = ValueStore::new();
    let data = vec![0u8; MAX_BLOB_BYTES_PER_VALUE.saturating_add(1)];
    let result = store.insert_blob(Bytes::from(data));
    match result {
        Err(CoreError::ResourceLimitExceeded {
            resource: "blob_bytes",
        }) => Ok(()),
        other => Err(format!("expected blob_bytes limit exceeded, got {other:?}")),
    }
}

// --- Blob reference after store drop is impossible (handle is Copy, resolved at access) ---

#[test]
fn value_store_blob_id_that_was_never_inserted_returns_out_of_bounds() -> Result<(), String> {
    let store = ValueStore::new();
    let result = store.blob(BlobId::new(0));
    match result {
        Err(CoreError::BlobOutOfBounds { blob }) if blob == BlobId::new(0) => Ok(()),
        other => Err(format!("expected BlobOutOfBounds, got {other:?}")),
    }
}

// --- List index exactly at list length is rejected ---

#[test]
fn value_store_list_index_at_exact_length_is_rejected() -> Result<(), String> {
    let mut store = ValueStore::new();
    let list_id = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    // Index 3 is exactly the length (0, 1, 2 are valid)
    let result = store.list_item(list_id, 3);
    match result {
        Err(CoreError::ListIndexOutOfBounds { index: 3 }) => Ok(()),
        other => Err(format!(
            "expected ListIndexOutOfBounds for index 3, got {other:?}"
        )),
    }
}

// --- Object field on a different object does not leak ---

#[test]
fn value_store_object_field_on_wrong_object_returns_not_found() -> Result<(), String> {
    let mut store = ValueStore::new();
    let key_only_in_first = SymbolId::new(42);
    let _obj1 = store
        .insert_object(
            vec![ObjectField {
                key: key_only_in_first,
                value: SlotValue::Bool(true),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let obj2 = store
        .insert_object(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    // Key 42 is in obj1 but not in obj2
    let result = store.object_field(obj2, key_only_in_first);
    match result {
        Err(CoreError::ObjectFieldNotFound { field }) if field == key_only_in_first => Ok(()),
        other => Err(format!("expected ObjectFieldNotFound, got {other:?}")),
    }
}

#[test]
fn value_store_rejected_symbol_over_max_does_not_mutate_arena() -> Result<(), String> {
    let mut store = ValueStore::new();
    let baseline = store
        .insert_symbol(Box::<str>::from("kept"))
        .map_err(|e| e.to_string())?;
    let too_large = "x".repeat(MAX_SYMBOL_BYTES_PER_VALUE.saturating_add(1));

    match store.insert_symbol(too_large.into_boxed_str()) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "symbol_bytes",
        }) => {}
        other => return Err(format!("expected symbol_bytes limit, got {other:?}")),
    }

    if store.symbol_count() != 1 {
        return Err(String::from("failed symbol insert must not change count"));
    }
    if store.symbol(baseline).map_err(|e| e.to_string())? != "kept" {
        return Err(String::from(
            "failed symbol insert must not corrupt payload",
        ));
    }
    Ok(())
}

#[test]
fn value_store_rejected_list_over_max_does_not_mutate_arena() -> Result<(), String> {
    let mut store = ValueStore::new();
    let baseline = store
        .insert_list(vec![SlotValue::I64(7)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let too_many = vec![SlotValue::Null; MAX_LIST_ITEMS_PER_VALUE.saturating_add(1)];

    match store.insert_list(too_many.into_boxed_slice()) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "list_items",
        }) => {}
        other => return Err(format!("expected list_items limit, got {other:?}")),
    }

    if store.list_count() != 1 {
        return Err(String::from("failed list insert must not change count"));
    }
    if store.list_item(baseline, 0).map_err(|e| e.to_string())? != SlotValue::I64(7) {
        return Err(String::from("failed list insert must not corrupt payload"));
    }
    Ok(())
}

#[test]
fn value_store_rejected_object_over_max_does_not_mutate_arena() -> Result<(), String> {
    let mut store = ValueStore::new();
    let key = SymbolId::new(11);
    let field = ObjectField {
        key,
        value: SlotValue::Bool(true),
        taint: Taint::Clean,
    };
    let baseline = store
        .insert_object(vec![field].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let too_many = vec![field; MAX_OBJECT_FIELDS_PER_VALUE.saturating_add(1)];

    match store.insert_object(too_many.into_boxed_slice()) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "object_fields",
        }) => {}
        other => return Err(format!("expected object_fields limit, got {other:?}")),
    }

    if store.object_count() != 1 {
        return Err(String::from("failed object insert must not change count"));
    }
    if store
        .object_field(baseline, key)
        .map_err(|e| e.to_string())?
        != SlotValue::Bool(true)
    {
        return Err(String::from("failed object insert must not corrupt index"));
    }
    Ok(())
}

#[test]
fn value_store_rejected_blob_over_max_does_not_mutate_arena() -> Result<(), String> {
    let mut store = ValueStore::new();
    let baseline = store
        .insert_blob(Bytes::from_static(b"kept"))
        .map_err(|e| e.to_string())?;
    let too_large = vec![0u8; MAX_BLOB_BYTES_PER_VALUE.saturating_add(1)];

    match store.insert_blob(Bytes::from(too_large)) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "blob_bytes",
        }) => {}
        other => return Err(format!("expected blob_bytes limit, got {other:?}")),
    }

    if store.blob_count() != 1 {
        return Err(String::from("failed blob insert must not change count"));
    }
    if store.blob(baseline).map_err(|e| e.to_string())? != b"kept" {
        return Err(String::from("failed blob insert must not corrupt payload"));
    }
    Ok(())
}

#[test]
fn value_store_exact_max_list_accesses_edges_without_unchecked_indexing() -> Result<(), String> {
    let mut store = ValueStore::new();
    let values = vec![SlotValue::I64(5); MAX_LIST_ITEMS_PER_VALUE];
    let id = store
        .insert_list(values.into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let last_index =
        u32::try_from(MAX_LIST_ITEMS_PER_VALUE.saturating_sub(1)).map_err(|e| e.to_string())?;
    let end_index = u32::try_from(MAX_LIST_ITEMS_PER_VALUE).map_err(|e| e.to_string())?;

    if store.list_item(id, 0).map_err(|e| e.to_string())? != SlotValue::I64(5) {
        return Err(String::from("first max-list item mismatch"));
    }
    if store.list_item(id, last_index).map_err(|e| e.to_string())? != SlotValue::I64(5) {
        return Err(String::from("last max-list item mismatch"));
    }
    if store.list_item(id, end_index) != Err(CoreError::ListIndexOutOfBounds { index: end_index }) {
        return Err(String::from("index exactly at max-list length must fail"));
    }
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore = "max-size object fixture is too slow under Miri")]
fn value_store_exact_max_object_preserves_duplicate_first_wins_index() -> Result<(), String> {
    let mut store = ValueStore::new();
    let duplicate_key = SymbolId::new(77);
    let unique_key = SymbolId::new(78);
    let mut fields = vec![
        ObjectField {
            key: duplicate_key,
            value: SlotValue::I64(1),
            taint: Taint::Clean,
        };
        MAX_OBJECT_FIELDS_PER_VALUE
    ];
    let last = fields
        .last_mut()
        .ok_or_else(|| String::from("max object fixture must contain fields"))?;
    *last = ObjectField {
        key: unique_key,
        value: SlotValue::I64(2),
        taint: Taint::Clean,
    };

    let id = store
        .insert_object(fields.into_boxed_slice())
        .map_err(|e| e.to_string())?;

    if store.object(id).map_err(|e| e.to_string())?.len() != MAX_OBJECT_FIELDS_PER_VALUE {
        return Err(String::from("max object field count mismatch"));
    }
    if store
        .object_field(id, duplicate_key)
        .map_err(|e| e.to_string())?
        != SlotValue::I64(1)
    {
        return Err(String::from(
            "duplicate object key must resolve to first value",
        ));
    }
    if store
        .object_field(id, unique_key)
        .map_err(|e| e.to_string())?
        != SlotValue::I64(2)
    {
        return Err(String::from("unique field at max object edge must resolve"));
    }
    Ok(())
}

// =========================================================================
// Phase 45 tests — ValueStore arena cap enforcement
// =========================================================================

#[test]
fn value_store_with_max_slots_allows_inserts_up_to_cap() -> Result<(), String> {
    let mut store = ValueStore::with_max_slots(3);
    assert_eq!(store.max_arena_entries(), 3);
    assert_eq!(store.total_arena_count(), 0);

    // Insert 3 entries total (1 symbol + 1 list + 1 blob) -- should succeed
    store
        .insert_symbol(Box::<str>::from("a"))
        .map_err(|e| e.to_string())?;
    assert_eq!(store.total_arena_count(), 1);

    store
        .insert_list(vec![SlotValue::Null].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    assert_eq!(store.total_arena_count(), 2);

    store
        .insert_blob(Bytes::from_static(b"x"))
        .map_err(|e| e.to_string())?;
    assert_eq!(store.total_arena_count(), 3);

    // 4th insert should fail
    match store.insert_symbol(Box::<str>::from("b")) {
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 3,
        }) => Ok(()),
        other => Err(format!("expected BudgetExceeded, got {other:?}")),
    }
}

#[test]
fn value_store_with_max_slots_one_rejects_second_insert() -> Result<(), String> {
    let mut store = ValueStore::with_max_slots(1);
    // First insert succeeds
    store
        .insert_symbol(Box::<str>::from("a"))
        .map_err(|e| e.to_string())?;
    assert_eq!(store.total_arena_count(), 1);
    // Second insert fails because cap is reached
    match store.insert_symbol(Box::<str>::from("b")) {
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 1,
        }) => Ok(()),
        other => Err(format!("expected BudgetExceeded, got {other:?}")),
    }
}

#[test]
fn value_store_new_has_no_cap_and_allows_unlimited_inserts() -> Result<(), String> {
    let mut store = ValueStore::new();
    assert_eq!(store.max_arena_entries(), 0);
    let mut expected_count = 0u64;
    for _ in 0..100u64 {
        store
            .insert_symbol(Box::<str>::from("s"))
            .map_err(|e| e.to_string())?;
        expected_count = expected_count
            .checked_add(1)
            .ok_or_else(|| String::from("count overflow"))?;
        if store.total_arena_count() != expected_count {
            return Err(String::from("total_arena_count mismatch"));
        }
    }
    assert_eq!(store.total_arena_count(), 100);
    Ok(())
}

#[test]
fn value_store_default_equals_new() {
    let default: ValueStore = Default::default();
    let constructed = ValueStore::new();
    assert_eq!(default, constructed);
}
