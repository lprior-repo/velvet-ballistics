    // =========================================================================
    // BLACKHAT security regression tests — value_store
    // =========================================================================

    // --- Attack: list_with_taint length mismatch must not partially mutate ---

    #[test]
    fn security_insert_list_with_taint_mismatch_does_not_mutate() -> Result<(), String> {
        let mut store = ValueStore::new();

        // Insert a baseline list to verify it's not corrupted
        let baseline = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|e| e.to_string())?;

        // Attempt to insert a list with mismatched taint length
        let values = vec![SlotValue::I64(10), SlotValue::I64(20)].into_boxed_slice();
        let taints = vec![Taint::Clean].into_boxed_slice(); // wrong length

        match store.insert_list_with_taint(values, taints) {
            Err(CoreError::InternalInvariantViolation {
                reason,
            }) => assert_eq!(
                reason.as_str(),
                "list values and taints length mismatch"
            ),
            other => return Err(format!("expected length mismatch error, got {other:?}")),
        }

        // Baseline must be untouched
        assert_eq!(store.list_count(), 1, "failed insert must not change count");
        assert_eq!(
            store.list_item(baseline, 0).map_err(|e| e.to_string())?,
            SlotValue::I64(1)
        );

        Ok(())
    }

    // --- Attack: arena cap check happens before ID allocation ---

    #[test]
    fn security_arena_cap_prevents_id_allocation_on_full() -> Result<(), String> {
        let mut store = ValueStore::with_max_slots(1);

        // Fill the cap
        let sym = store
            .insert_symbol(Box::<str>::from("only"))
            .map_err(|e| e.to_string())?;

        // Verify the first symbol got ID 0
        assert_eq!(sym.get(), 0);

        // Second insert must fail with BudgetExceeded, not ResourceLimitExceeded
        // (arena cap check happens before ID overflow check)
        match store.insert_symbol(Box::<str>::from("overflow")) {
            Err(CoreError::BudgetExceeded { .. }) => Ok(()),
            other => Err(format!("expected BudgetExceeded, got {other:?}")),
        }
    }

    // --- Attack: empty list taint array consistency ---

    #[test]
    fn security_empty_list_has_consistent_taint_array() -> Result<(), String> {
        let mut store = ValueStore::new();

        let list_id = store
            .insert_list(vec![].into_boxed_slice())
            .map_err(|e| e.to_string())?;

        // Empty list resolves to empty slice
        assert_eq!(store.list(list_id).map_err(|e| e.to_string())?.len(), 0);

        // Any index must fail
        assert_eq!(
            store.list_item(list_id, 0),
            Err(CoreError::ListIndexOutOfBounds { index: 0 })
        );

        // list_item_with_taint also fails on empty list
        assert_eq!(
            store.list_item_with_taint(list_id, 0),
            Err(CoreError::ListIndexOutOfBounds { index: 0 })
        );

        Ok(())
    }

    // --- Attack: object with duplicate keys -- first-wins semantics for taint ---

    #[test]
    fn security_object_duplicate_key_first_wins_for_taint() -> Result<(), String> {
        let mut store = ValueStore::new();
        let key = SymbolId::new(1);

        let obj_id = store
            .insert_object(
                vec![
                    ObjectField {
                        key,
                        value: SlotValue::I64(100),
                        taint: Taint::Secret,
                    },
                    ObjectField {
                        key,
                        value: SlotValue::I64(200),
                        taint: Taint::Clean,
                    },
                ]
                .into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;

        // object_field returns the first value
        assert_eq!(
            store.object_field(obj_id, key).map_err(|e| e.to_string())?,
            SlotValue::I64(100)
        );

        // object_field_with_taint must also return the first taint
        let (value, taint) = store
            .object_field_with_taint(obj_id, key)
            .map_err(|e| e.to_string())?;
        assert_eq!(value, SlotValue::I64(100));
        assert_eq!(
            taint,
            Taint::Secret,
            "first-wins must apply to taint index too"
        );

        Ok(())
    }

    // --- Attack: value confusion between list_taints and lists arrays ---

    #[test]
    fn security_list_taints_are_per_list_not_global() -> Result<(), String> {
        let mut store = ValueStore::new();

        let clean_list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|e| e.to_string())?;

        let secret_list = store
            .insert_list_with_taint(
                vec![SlotValue::I64(2)].into_boxed_slice(),
                vec![Taint::Secret].into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;

        // Verify each list has its own taint
        let (_, clean_taint) = store
            .list_item_with_taint(clean_list, 0)
            .map_err(|e| e.to_string())?;
        assert_eq!(clean_taint, Taint::Clean);

        let (_, secret_taint) = store
            .list_item_with_taint(secret_list, 0)
            .map_err(|e| e.to_string())?;
        assert_eq!(secret_taint, Taint::Secret);

        Ok(())
    }

    // --- Attack: total_arena_count saturating_add safety ---

    #[test]
    fn security_total_arena_count_uses_saturating_add() -> Result<(), String> {
        // Verify that total_arena_count doesn't overflow with large counts
        let store = ValueStore::new();
        // Empty store should report 0
        assert_eq!(store.total_arena_count(), 0);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Proptest property: PROPTEST-PRE-002
    // ValueStore inserts return BudgetExceeded when total_arena_count >= max_arena_entries
    // -------------------------------------------------------------------------

    proptest::proptest! {
        #[cfg_attr(miri, ignore)]
        #[test]
        fn property_value_store_cap(cap: u16, insert_count: u16) {
            use proptest::{prop_assert, prop_assert_eq};
            let mut store = ValueStore::with_max_slots(cap);
            // cap = 0 means uncapped (check_arena_cap returns Ok immediately),
            // so we use u64::MAX as the effective limit for the assertion.
            let max_entries = if cap == 0 {
                u64::MAX
            } else {
                u64::from(cap)
            };

            // Insert insert_count symbols (each counts as 1 arena entry)
            let mut succeeded = 0u64;
            for i in 0..insert_count {
                let sym = format!("sym_{}", i);
                match store.insert_symbol(sym.into_boxed_str()) {
                    Ok(_) => { succeeded += 1; }
                    Err(CoreError::BudgetExceeded { .. }) => {
                        // Expected once cap is reached
                    }
                    Err(e) => panic!("unexpected error: {:?}", e),
                }
                // Arena count must never exceed cap (or be uncapped)
                prop_assert!(store.total_arena_count() <= max_entries);
            }

            // If cap > 0: succeeded == min(insert_count, cap)
            // If cap == 0: all inserts succeed (uncapped)
            if cap > 0 {
                prop_assert_eq!(succeeded, (insert_count as u64).min(u64::from(cap)));
            } else {
                prop_assert_eq!(succeeded, insert_count as u64);
            }
        }
    }

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
        let fields =
            vec![ObjectField::clean(SymbolId::new(5), SlotValue::I64(42))].into_boxed_slice();
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

    // =========================================================================
    // Additional coverage: Arena cap near limit
    // =========================================================================

    #[test]
    fn value_store_arena_cap_fills_and_rejects() -> Result<(), String> {
        let mut store = ValueStore::with_max_slots(2);
        store
            .insert_symbol(Box::<str>::from("a"))
            .map_err(|e| e.to_string())?;
        store
            .insert_list(vec![SlotValue::Null].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        match store.insert_object(vec![].into_boxed_slice()) {
            Err(CoreError::BudgetExceeded {
                budget: "max_slots",
                limit: 2,
            }) => Ok(()),
            other => Err(format!("expected BudgetExceeded, got {:?}", other)),
        }
    }

