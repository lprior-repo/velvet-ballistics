
    #[test]
    fn insert_list_rejects_payload_over_hard_bound() -> Result<(), String> {
        let mut store = ValueStore::new();
        let values =
            vec![SlotValue::Null; MAX_LIST_ITEMS_PER_VALUE.saturating_add(1)].into_boxed_slice();

        match store.insert_list(values) {
            Err(CoreError::ResourceLimitExceeded {
                resource: "list_items",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn insert_object_rejects_payload_over_hard_bound() -> Result<(), String> {
        let mut store = ValueStore::new();
        let field = ObjectField {
            key: SymbolId::new(0),
            value: SlotValue::Null,
            taint: Taint::Clean,
        };
        let fields = vec![field; MAX_OBJECT_FIELDS_PER_VALUE.saturating_add(1)].into_boxed_slice();

        match store.insert_object(fields) {
            Err(CoreError::ResourceLimitExceeded {
                resource: "object_fields",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn insert_symbol_rejects_payload_over_hard_bound() -> Result<(), String> {
        let mut store = ValueStore::new();
        let value = "x".repeat(MAX_SYMBOL_BYTES_PER_VALUE.saturating_add(1));

        match store.insert_symbol(value.into_boxed_str()) {
            Err(CoreError::ResourceLimitExceeded {
                resource: "symbol_bytes",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn insert_blob_rejects_payload_over_hard_bound() -> Result<(), String> {
        let mut store = ValueStore::new();
        let bytes = vec![0u8; MAX_BLOB_BYTES_PER_VALUE.saturating_add(1)];

        match store.insert_blob(Bytes::from(bytes)) {
            Err(CoreError::ResourceLimitExceeded {
                resource: "blob_bytes",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn symbol_and_blob_accessors_return_payloads() -> Result<(), String> {
        let mut store = ValueStore::new();
        let symbol = store
            .insert_symbol(Box::<str>::from("alpha"))
            .map_err(|error| error.to_string())?;
        let blob = store
            .insert_blob(Bytes::from_static(b"payload"))
            .map_err(|error| error.to_string())?;

        if store.symbol(symbol).map_err(|error| error.to_string())? != "alpha" {
            return Err(String::from("unexpected symbol payload"));
        }
        if store.blob(blob).map_err(|error| error.to_string())? != b"payload" {
            return Err(String::from("unexpected blob payload"));
        }
        Ok(())
    }

    #[test]
    fn arena_accessors_report_handle_bounds() -> Result<(), String> {
        let store = ValueStore::new();

        if store.symbol(SymbolId::new(0))
            != Err(CoreError::SymbolOutOfBounds {
                symbol: SymbolId::new(0),
            })
        {
            return Err(String::from("expected symbol bounds error"));
        }
        if store.list(ListId::new(0))
            != Err(CoreError::ListOutOfBounds {
                list: ListId::new(0),
            })
        {
            return Err(String::from("expected list bounds error"));
        }
        if store.object(ObjectId::new(0))
            != Err(CoreError::ObjectOutOfBounds {
                object: ObjectId::new(0),
            })
        {
            return Err(String::from("expected object bounds error"));
        }
        if store.blob(BlobId::new(0))
            != Err(CoreError::BlobOutOfBounds {
                blob: BlobId::new(0),
            })
        {
            return Err(String::from("expected blob bounds error"));
        }
        Ok(())
    }

    #[test]
    fn list_item_and_object_field_accessors_are_checked() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(10)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let object = store
            .insert_object(
                vec![ObjectField {
                    key: SymbolId::new(4),
                    value: SlotValue::Bool(true),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .map_err(|error| error.to_string())?;

        if store
            .list_item(list, 0)
            .map_err(|error| error.to_string())?
            != SlotValue::I64(10)
        {
            return Err(String::from("unexpected list item"));
        }
        if store
            .object_field(object, SymbolId::new(4))
            .map_err(|error| error.to_string())?
            != SlotValue::Bool(true)
        {
            return Err(String::from("unexpected object field"));
        }
        if store.list_item(list, 1) != Err(CoreError::ListIndexOutOfBounds { index: 1 }) {
            return Err(String::from("expected list item bounds error"));
        }
        if store.object_field(object, SymbolId::new(5))
            != Err(CoreError::ObjectFieldNotFound {
                field: SymbolId::new(5),
            })
        {
            return Err(String::from("expected missing object field error"));
        }
        Ok(())
    }

    // =========================================================================
    // Adversarial BDD tests — ValueStore handle corruption and boundary attacks
    // =========================================================================

    #[test]
    fn value_store_empty_store_rejects_symbol_id_zero() {
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
    fn value_store_empty_store_rejects_list_id_zero() {
        let store = ValueStore::new();
        let result = store.list(ListId::new(0));
        assert_eq!(
            result,
            Err(CoreError::ListOutOfBounds {
                list: ListId::new(0),
            })
        );
    }

    #[test]
    fn value_store_empty_store_rejects_object_id_zero() {
        let store = ValueStore::new();
        let result = store.object(ObjectId::new(0));
        assert_eq!(
            result,
            Err(CoreError::ObjectOutOfBounds {
                object: ObjectId::new(0),
            })
        );
    }

    #[test]
    fn value_store_empty_store_rejects_blob_id_zero() {
        let store = ValueStore::new();
        let result = store.blob(BlobId::new(0));
        assert_eq!(
            result,
            Err(CoreError::BlobOutOfBounds {
                blob: BlobId::new(0),
            })
        );
    }

    #[test]
    fn value_store_symbol_handle_high_id_rejected() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id0 = store
            .insert_symbol(Box::<str>::from("first"))
            .map_err(|e| e.to_string())?;
        let result = store.symbol(SymbolId::new(1));
        if result
            != Err(CoreError::SymbolOutOfBounds {
                symbol: SymbolId::new(1),
            })
        {
            return Err(String::from("id 1 must fail when only id 0 exists"));
        }
        if store.symbol(id0).map_err(|e| e.to_string())? != "first" {
            return Err(String::from("valid id must resolve"));
        }
        Ok(())
    }

    #[test]
    fn value_store_list_handle_high_id_rejected() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id0 = store
            .insert_list(vec![SlotValue::Null].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        if store.list(ListId::new(1))
            != Err(CoreError::ListOutOfBounds {
                list: ListId::new(1),
            })
        {
            return Err(String::from("expected list bounds error for id 1"));
        }
        let items = store.list(id0).map_err(|e| e.to_string())?;
        if items.len() != 1 {
            return Err(String::from("expected 1 item"));
        }
        Ok(())
    }

    #[test]
    fn value_store_object_handle_high_id_rejected() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id0 = store
            .insert_object(
                vec![ObjectField {
                    key: SymbolId::new(0),
                    value: SlotValue::Null,
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;
        if store.object(ObjectId::new(1))
            != Err(CoreError::ObjectOutOfBounds {
                object: ObjectId::new(1),
            })
        {
            return Err(String::from("expected object bounds error for id 1"));
        }
        let fields = store.object(id0).map_err(|e| e.to_string())?;
        if fields.len() != 1 {
            return Err(String::from("expected 1 field"));
        }
        Ok(())
    }

    #[test]
    fn value_store_blob_handle_high_id_rejected() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id0 = store
            .insert_blob(Bytes::from_static(b"a"))
            .map_err(|e| e.to_string())?;
        if store.blob(BlobId::new(1))
            != Err(CoreError::BlobOutOfBounds {
                blob: BlobId::new(1),
            })
        {
            return Err(String::from("expected blob bounds error for id 1"));
        }
        if store.blob(id0).map_err(|e| e.to_string())? != b"a" {
            return Err(String::from("valid blob must resolve"));
        }
        Ok(())
    }

    #[test]
    fn value_store_list_item_index_zero_on_empty_list_fails() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list_id = store
            .insert_list(vec![].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = store.list_item(list_id, 0);
        if result != Err(CoreError::ListIndexOutOfBounds { index: 0 }) {
            return Err(String::from("empty list must reject index 0"));
        }
        Ok(())
    }

    #[test]
    fn value_store_list_item_max_u32_index_rejected() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list_id = store
            .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = store.list_item(list_id, u32::MAX);
        if result != Err(CoreError::ListIndexOutOfBounds { index: u32::MAX }) {
            return Err(String::from("u32::MAX index must be rejected"));
        }
        Ok(())
    }

    #[test]
    fn value_store_object_field_missing_key_returns_not_found() -> Result<(), String> {
        let mut store = ValueStore::new();
        let key_present = SymbolId::new(10);
        let key_absent = SymbolId::new(99);
        let obj_id = store
            .insert_object(
                vec![ObjectField {
                    key: key_present,
                    value: SlotValue::Bool(true),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;
        if store.object_field(obj_id, key_absent)
            != Err(CoreError::ObjectFieldNotFound { field: key_absent })
        {
            return Err(String::from(
                "missing field must return ObjectFieldNotFound",
            ));
        }
        if store
            .object_field(obj_id, key_present)
            .map_err(|e| e.to_string())?
            != SlotValue::Bool(true)
        {
            return Err(String::from("present key must resolve"));
        }
        Ok(())
    }

    #[test]
    fn value_store_object_field_returns_first_duplicate_key() -> Result<(), String> {
        // If the same key appears twice, linear scan finds the FIRST one.
        let mut store = ValueStore::new();
        let dup_key = SymbolId::new(1);
        let obj_id = store
            .insert_object(
                vec![
                    ObjectField {
                        key: dup_key,
                        value: SlotValue::I64(100),
                        taint: Taint::Clean,
                    },
                    ObjectField {
                        key: dup_key,
                        value: SlotValue::I64(200),
                        taint: Taint::Clean,
                    },
                ]
                .into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;
        if store
            .object_field(obj_id, dup_key)
            .map_err(|e| e.to_string())?
            != SlotValue::I64(100)
        {
            return Err(String::from("linear scan must return first match"));
        }
        Ok(())
    }

    #[test]
    fn value_store_insert_symbol_empty_string_is_valid() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id = store
            .insert_symbol(Box::<str>::from(""))
            .map_err(|e| e.to_string())?;
        if !store.symbol(id).map_err(|e| e.to_string())?.is_empty() {
            return Err(String::from("empty symbol must round-trip"));
        }
        Ok(())
    }

    #[test]
    fn value_store_insert_list_empty_is_valid() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id = store
            .insert_list(vec![].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let items = store.list(id).map_err(|e| e.to_string())?;
        if !items.is_empty() {
            return Err(String::from("empty list must be empty"));
        }
        Ok(())
    }

    #[test]
    fn value_store_insert_object_empty_is_valid() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id = store
            .insert_object(vec![].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let fields = store.object(id).map_err(|e| e.to_string())?;
        if !fields.is_empty() {
            return Err(String::from("empty object must be empty"));
        }
        Ok(())
    }

    #[test]
    fn value_store_insert_blob_empty_is_valid() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id = store.insert_blob(Bytes::new()).map_err(|e| e.to_string())?;
        if store.blob(id).map_err(|e| e.to_string())?.is_empty() {
            Ok(())
        } else {
            Err(String::from("empty blob must be empty"))
        }
    }

    #[test]
    fn value_store_counts_track_insertions() -> Result<(), String> {
        let mut store = ValueStore::new();
        if store.symbol_count() != 0 {
            return Err(String::from("initial symbol count must be 0"));
        }
        if store.list_count() != 0 {
            return Err(String::from("initial list count must be 0"));
        }
        if store.object_count() != 0 {
            return Err(String::from("initial object count must be 0"));
        }
        if store.blob_count() != 0 {
            return Err(String::from("initial blob count must be 0"));
        }

        store
            .insert_symbol(Box::<str>::from("a"))
            .map_err(|e| e.to_string())?;
        store
            .insert_symbol(Box::<str>::from("b"))
            .map_err(|e| e.to_string())?;
        if store.symbol_count() != 2 {
            return Err(String::from("symbol count must be 2 after two inserts"));
        }

        store
            .insert_list(vec![SlotValue::Null].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        if store.list_count() != 1 {
            return Err(String::from("list count must be 1 after one insert"));
        }

        store
            .insert_object(
                vec![ObjectField {
                    key: SymbolId::new(0),
                    value: SlotValue::Null,
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;
        if store.object_count() != 1 {
            return Err(String::from("object count must be 1 after one insert"));
        }

        store
            .insert_blob(Bytes::from_static(b"x"))
            .map_err(|e| e.to_string())?;
        if store.blob_count() != 1 {
            return Err(String::from("blob count must be 1 after one insert"));
        }
        Ok(())
    }

    #[test]
    fn value_store_symbol_at_exact_max_length_is_accepted() -> Result<(), String> {
        let mut store = ValueStore::new();
        let value = "x".repeat(MAX_SYMBOL_BYTES_PER_VALUE);
        let id = store
            .insert_symbol(value.into_boxed_str())
            .map_err(|e| e.to_string())?;
        let retrieved = store.symbol(id).map_err(|e| e.to_string())?;
        if retrieved.len() != MAX_SYMBOL_BYTES_PER_VALUE {
            return Err(String::from("symbol length mismatch"));
        }
        Ok(())
    }

    #[test]
    fn value_store_list_at_exact_max_length_is_accepted() -> Result<(), String> {
        let mut store = ValueStore::new();
        let values = vec![SlotValue::Null; MAX_LIST_ITEMS_PER_VALUE];
        let id = store
            .insert_list(values.into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let retrieved = store.list(id).map_err(|e| e.to_string())?;
        if retrieved.len() != MAX_LIST_ITEMS_PER_VALUE {
            return Err(String::from("list length mismatch"));
        }
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore = "max-size object fixture is too slow under Miri")]
    fn value_store_object_at_exact_max_fields_is_accepted() -> Result<(), String> {
        let mut store = ValueStore::new();
        let field = ObjectField {
            key: SymbolId::new(0),
            value: SlotValue::Null,
            taint: Taint::Clean,
        };
        let fields = vec![field; MAX_OBJECT_FIELDS_PER_VALUE];
        let id = store
            .insert_object(fields.into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let retrieved = store.object(id).map_err(|e| e.to_string())?;
        if retrieved.len() != MAX_OBJECT_FIELDS_PER_VALUE {
            return Err(String::from("object field count mismatch"));
        }
        Ok(())
    }

    #[test]
    fn value_store_blob_at_exact_max_bytes_is_accepted() -> Result<(), String> {
        let mut store = ValueStore::new();
        let data = vec![0u8; MAX_BLOB_BYTES_PER_VALUE];
        let id = store
            .insert_blob(Bytes::from(data))
            .map_err(|e| e.to_string())?;
        let retrieved = store.blob(id).map_err(|e| e.to_string())?;
        if retrieved.len() != MAX_BLOB_BYTES_PER_VALUE {
            return Err(String::from("blob length mismatch"));
        }
        Ok(())
    }

    #[test]
    fn value_store_sequential_ids_are_monotonic() -> Result<(), String> {
        let mut store = ValueStore::new();
        let s0 = store
            .insert_symbol(Box::<str>::from("a"))
            .map_err(|e| e.to_string())?;
        let s1 = store
            .insert_symbol(Box::<str>::from("b"))
            .map_err(|e| e.to_string())?;
        let s2 = store
            .insert_symbol(Box::<str>::from("c"))
            .map_err(|e| e.to_string())?;
        if s0.get() != 0 || s1.get() != 1 || s2.get() != 2 {
            return Err(String::from("symbol ids must be 0, 1, 2"));
        }

        let l0 = store
            .insert_list(vec![].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let l1 = store
            .insert_list(vec![SlotValue::Null].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        if l0.get() != 0 || l1.get() != 1 {
            return Err(String::from("list ids must be 0, 1"));
        }

        let b0 = store.insert_blob(Bytes::new()).map_err(|e| e.to_string())?;
        let b1 = store
            .insert_blob(Bytes::from_static(b"z"))
            .map_err(|e| e.to_string())?;
        if b0.get() != 0 || b1.get() != 1 {
            return Err(String::from("blob ids must be 0, 1"));
        }
        Ok(())
    }

    #[test]
    fn value_store_default_is_same_as_new() {
        let default: ValueStore = Default::default();
        let constructed = ValueStore::new();
        assert_eq!(default, constructed);
    }

    #[test]
    fn value_store_clone_is_equal() -> Result<(), String> {
        let mut store = ValueStore::new();
        store
            .insert_symbol(Box::<str>::from("hello"))
            .map_err(|e| e.to_string())?;
        let cloned = store.clone();
        if store != cloned {
            return Err(String::from("clone must be equal"));
        }
        Ok(())
    }

    #[test]
    fn value_store_list_with_mixed_slot_value_types() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list_id = store
            .insert_list(
                vec![
                    SlotValue::Null,
                    SlotValue::Bool(true),
                    SlotValue::I64(-42),
                    SlotValue::List(ListId::new(99)),
                    SlotValue::Object(ObjectId::new(7)),
                    SlotValue::Blob(BlobId::new(3)),
                ]
                .into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;
        if store.list_item(list_id, 0).map_err(|e| e.to_string())? != SlotValue::Null {
            return Err(String::from("index 0 mismatch"));
        }
        if store.list_item(list_id, 1).map_err(|e| e.to_string())? != SlotValue::Bool(true) {
            return Err(String::from("index 1 mismatch"));
        }
        if store.list_item(list_id, 2).map_err(|e| e.to_string())? != SlotValue::I64(-42) {
            return Err(String::from("index 2 mismatch"));
        }
        if store.list_item(list_id, 3).map_err(|e| e.to_string())?
            != SlotValue::List(ListId::new(99))
        {
            return Err(String::from("index 3 mismatch"));
        }
        if store.list_item(list_id, 4).map_err(|e| e.to_string())?
            != SlotValue::Object(ObjectId::new(7))
        {
            return Err(String::from("index 4 mismatch"));
        }
        if store.list_item(list_id, 5).map_err(|e| e.to_string())?
            != SlotValue::Blob(BlobId::new(3))
        {
            return Err(String::from("index 5 mismatch"));
        }
        Ok(())
    }

    #[test]
    fn value_store_object_field_linear_scan_respects_insertion_order() -> Result<(), String> {
        // Object with three fields; the second has the same key as the first.
        // Linear scan must find the first occurrence.
        let mut store = ValueStore::new();
        let shared_key = SymbolId::new(5);
        let unique_key = SymbolId::new(10);
        let obj_id = store
            .insert_object(
                vec![
                    ObjectField {
                        key: shared_key,
                        value: SlotValue::I64(1),
                        taint: Taint::Clean,
                    },
                    ObjectField {
                        key: shared_key,
                        value: SlotValue::I64(2),
                        taint: Taint::Clean,
                    },
                    ObjectField {
                        key: unique_key,
                        value: SlotValue::I64(3),
                        taint: Taint::Clean,
                    },
                ]
                .into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;
        if store
            .object_field(obj_id, shared_key)
            .map_err(|e| e.to_string())?
            != SlotValue::I64(1)
        {
            return Err(String::from("shared_key must resolve to first occurrence"));
        }
        if store
            .object_field(obj_id, unique_key)
            .map_err(|e| e.to_string())?
            != SlotValue::I64(3)
        {
            return Err(String::from("unique_key must resolve"));
        }
        Ok(())
    }

