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
    fn value_store_exact_max_list_accesses_edges_without_unchecked_indexing() -> Result<(), String>
    {
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
        if store.list_item(id, end_index)
            != Err(CoreError::ListIndexOutOfBounds { index: end_index })
        {
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
            let slot = crate::ids::SlotIdx::new(
                u16::try_from(i).map_err(|_| String::from("index overflow"))?,
            );
            frame
                .write_slot_with_taint(slot, SlotValue::I64(i64::try_from(i).unwrap_or(0)), *taint)
                .map_err(|e| e.to_string())?;
        }

        // Verify all slot/taint pairs are consistent
        for (i, expected_taint) in taints.iter().enumerate() {
            let slot = crate::ids::SlotIdx::new(
                u16::try_from(i).map_err(|_| String::from("index overflow"))?,
            );
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

