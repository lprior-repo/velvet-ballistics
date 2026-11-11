// =========================================================================
// Security regression tests — handle forgery, overflow, type confusion
// =========================================================================

// --- Attack 1: Handle forgery (forged IDs must fail safely, not panic) ---

#[test]
fn security_forged_symbol_id_max_u32_returns_error_not_panic() {
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
fn security_forged_list_id_max_u32_returns_error_not_panic() {
    let store = ValueStore::new();
    let result = store.list(ListId::new(u32::MAX));
    assert_eq!(
        result,
        Err(CoreError::ListOutOfBounds {
            list: ListId::new(u32::MAX),
        })
    );
}

#[test]
fn security_forged_object_id_max_u32_returns_error_not_panic() {
    let store = ValueStore::new();
    let result = store.object(ObjectId::new(u32::MAX));
    assert_eq!(
        result,
        Err(CoreError::ObjectOutOfBounds {
            object: ObjectId::new(u32::MAX),
        })
    );
}

#[test]
fn security_forged_blob_id_max_u64_returns_error_not_panic() {
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
fn security_forged_id_one_past_last_insert_returns_error() -> Result<(), String> {
    let mut store = ValueStore::new();
    let sym = store
        .insert_symbol(Box::<str>::from("a"))
        .map_err(|e| e.to_string())?;
    let list = store
        .insert_list(vec![SlotValue::Null].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let obj = store
        .insert_object(vec![].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let blob = store
        .insert_blob(Bytes::from_static(b"x"))
        .map_err(|e| e.to_string())?;

    // Verify valid handles work
    assert_eq!(store.symbol(sym).map_err(|e| e.to_string())?, "a");
    assert_eq!(store.list(list).map_err(|e| e.to_string())?.len(), 1);
    assert_eq!(store.object(obj).map_err(|e| e.to_string())?.len(), 0);
    assert_eq!(store.blob(blob).map_err(|e| e.to_string())?.len(), 1);

    // Forged one-past handles must fail
    let forged_sym = SymbolId::new(sym.get().saturating_add(1));
    assert_eq!(
        store.symbol(forged_sym),
        Err(CoreError::SymbolOutOfBounds { symbol: forged_sym })
    );

    let forged_list = ListId::new(list.get().saturating_add(1));
    assert_eq!(
        store.list(forged_list),
        Err(CoreError::ListOutOfBounds { list: forged_list })
    );

    let forged_obj = ObjectId::new(obj.get().saturating_add(1));
    assert_eq!(
        store.object(forged_obj),
        Err(CoreError::ObjectOutOfBounds { object: forged_obj })
    );

    let forged_blob = BlobId::new(blob.get().saturating_add(1));
    assert_eq!(
        store.blob(forged_blob),
        Err(CoreError::BlobOutOfBounds { blob: forged_blob })
    );

    Ok(())
}

// --- Attack 2: Arena overflow cannot corrupt existing entries ---

#[test]
fn security_arena_overflow_preserves_existing_entries() -> Result<(), String> {
    let mut store = ValueStore::with_max_slots(2);
    let sym = store
        .insert_symbol(Box::<str>::from("preserved"))
        .map_err(|e| e.to_string())?;
    let list = store
        .insert_list(vec![SlotValue::I64(42)].into_boxed_slice())
        .map_err(|e| e.to_string())?;

    // Third insert must fail
    assert_eq!(
        store.insert_symbol(Box::<str>::from("overflow")),
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 2,
        })
    );

    // Existing entries must be untouched
    assert_eq!(store.symbol(sym).map_err(|e| e.to_string())?, "preserved");
    assert_eq!(
        store.list_item(list, 0).map_err(|e| e.to_string())?,
        SlotValue::I64(42)
    );
    assert_eq!(store.total_arena_count(), 2);

    Ok(())
}

// --- Attack 5: Type confusion (cross-arena index has same numeric value) ---

#[test]
fn security_symbol_id_zero_does_not_leak_list_data() -> Result<(), String> {
    let mut store = ValueStore::new();
    // Insert a symbol at index 0
    let sym_id = store
        .insert_symbol(Box::<str>::from("symbol_zero"))
        .map_err(|e| e.to_string())?;
    assert_eq!(sym_id.get(), 0);

    // Insert a list at index 0
    let list_id = store
        .insert_list(vec![SlotValue::Bool(true)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    assert_eq!(list_id.get(), 0);

    // SymbolId(0) resolves to the symbol, not the list
    assert_eq!(
        store.symbol(sym_id).map_err(|e| e.to_string())?,
        "symbol_zero"
    );

    // ListId(0) resolves to the list, not the symbol
    assert_eq!(
        store.list_item(list_id, 0).map_err(|e| e.to_string())?,
        SlotValue::Bool(true)
    );

    Ok(())
}

#[test]
fn security_object_field_index_and_objects_vec_stay_synchronized() -> Result<(), String> {
    let mut store = ValueStore::new();
    let key = SymbolId::new(7);

    // Insert first object
    let obj0 = store
        .insert_object(
            vec![ObjectField {
                key,
                value: SlotValue::I64(100),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;

    // Insert second object
    let obj1 = store
        .insert_object(
            vec![ObjectField {
                key,
                value: SlotValue::I64(200),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;

    // Each object must resolve its own field
    assert_eq!(
        store.object_field(obj0, key).map_err(|e| e.to_string())?,
        SlotValue::I64(100)
    );
    assert_eq!(
        store.object_field(obj1, key).map_err(|e| e.to_string())?,
        SlotValue::I64(200)
    );

    // Raw field slices must also be distinct
    let fields0 = store.object(obj0).map_err(|e| e.to_string())?;
    let fields1 = store.object(obj1).map_err(|e| e.to_string())?;
    assert_eq!(fields0.len(), 1);
    assert_eq!(fields1.len(), 1);
    assert_eq!(fields0[0].value, SlotValue::I64(100));
    assert_eq!(fields1[0].value, SlotValue::I64(200));

    Ok(())
}

// --- Attack 7: Taint array length invariants (verified via RunFrame, not ValueStore) ---

#[test]
fn security_write_slot_with_taint_maintains_same_length_arrays() -> Result<(), String> {
    use crate::frame::RunFrame;

    let mut frame = RunFrame::new(crate::ids::RunId::new(1), crate::ids::StepIdx::ZERO, 2, 4)
        .map_err(|e| e.to_string())?;

    // Write all slots with different taint levels
    let taints = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Clean,
    ];
    for (i, taint) in taints.iter().enumerate() {
        let slot =
            crate::ids::SlotIdx::new(u16::try_from(i).map_err(|_| String::from("index overflow"))?);
        frame
            .write_slot_with_taint(slot, SlotValue::I64(i64::try_from(i).unwrap_or(0)), *taint)
            .map_err(|e| e.to_string())?;
    }

    // Verify all slot/taint pairs are consistent
    for (i, expected_taint) in taints.iter().enumerate() {
        let slot =
            crate::ids::SlotIdx::new(u16::try_from(i).map_err(|_| String::from("index overflow"))?);
        assert_eq!(
            frame.read_taint(slot).map_err(|e| e.to_string())?,
            *expected_taint,
            "taint mismatch at slot {i}"
        );
        let slot_val = frame.read_slot(slot).map_err(|e| e.to_string())?;
        assert_eq!(
            *slot_val,
            SlotValue::I64(i64::try_from(i).unwrap_or(0)),
            "slot value mismatch at index {i}"
        );
    }

    Ok(())
}

// --- Defensive: list_item on forged list handle ---

#[test]
fn security_list_item_on_forged_list_id_returns_list_out_of_bounds() {
    let store = ValueStore::new();
    let result = store.list_item(ListId::new(0), 0);
    assert_eq!(
        result,
        Err(CoreError::ListOutOfBounds {
            list: ListId::new(0),
        })
    );
}

// --- Defensive: object_field on forged object handle ---

#[test]
fn security_object_field_on_forged_object_id_returns_object_out_of_bounds() {
    let store = ValueStore::new();
    let result = store.object_field(ObjectId::new(0), SymbolId::new(0));
    assert_eq!(
        result,
        Err(CoreError::ObjectOutOfBounds {
            object: ObjectId::new(0),
        })
    );
}

// --- Defensive: large arena cap edge case ---

#[test]
fn security_with_max_slots_u16_max_allows_inserts() -> Result<(), String> {
    let mut store = ValueStore::with_max_slots(u16::MAX);
    assert_eq!(store.max_arena_entries(), u64::from(u16::MAX));
    // Insert should succeed -- arena is not full
    store
        .insert_symbol(Box::<str>::from("ok"))
        .map_err(|e| e.to_string())?;
    assert_eq!(store.total_arena_count(), 1);
    Ok(())
}
