    // =========================================================================
    // Additional coverage: ObjectField accessors
    // =========================================================================

    #[test]
    fn object_field_clean_creates_clean_taint() -> Result<(), String> {
        let field = ObjectField::clean(SymbolId::new(1), SlotValue::I64(42));
        assert_eq!(field.taint, Taint::Clean);
        assert_eq!(field.key, SymbolId::new(1));
        assert_eq!(field.value, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn object_field_with_taint_preserves_taint() -> Result<(), String> {
        let field = ObjectField::with_taint(SymbolId::new(2), SlotValue::Bool(true), Taint::Secret);
        assert_eq!(field.taint, Taint::Secret);
        assert_eq!(field.key, SymbolId::new(2));
        assert_eq!(field.value, SlotValue::Bool(true));
        Ok(())
    }

    // =========================================================================
    // Additional coverage: Debug formatting
    // =========================================================================

    #[test]
    fn value_store_debug_format() -> Result<(), String> {
        let store = ValueStore::new();
        let debug = format!("{:?}", store);
        assert!(debug.contains("ValueStore"));
        Ok(())
    }

    #[test]
    fn object_field_debug_format() -> Result<(), String> {
        let field = ObjectField::clean(SymbolId::new(1), SlotValue::Null);
        let debug = format!("{:?}", field);
        assert!(debug.contains("ObjectField"));
        Ok(())
    }

    // =========================================================================
    // Additional coverage: checked_len_to_u64 via store operations
    // =========================================================================

    #[test]
    fn value_store_symbol_id_allocates_sequential() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id0 = store
            .insert_symbol(Box::<str>::from("a"))
            .map_err(|e| e.to_string())?;
        let id1 = store
            .insert_symbol(Box::<str>::from("b"))
            .map_err(|e| e.to_string())?;
        assert_eq!(id0.get(), 0);
        assert_eq!(id1.get(), 1);
        Ok(())
    }

    #[test]
    fn value_store_list_id_allocates_sequential() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id0 = store
            .insert_list(vec![].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let id1 = store
            .insert_list(vec![SlotValue::Null].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        assert_eq!(id0.get(), 0);
        assert_eq!(id1.get(), 1);
        Ok(())
    }

    #[test]
    fn value_store_object_id_allocates_sequential() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id0 = store
            .insert_object(vec![].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let id1 = store
            .insert_object(vec![].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        assert_eq!(id0.get(), 0);
        assert_eq!(id1.get(), 1);
        Ok(())
    }

    #[test]
    fn value_store_blob_id_allocates_sequential() -> Result<(), String> {
        let mut store = ValueStore::new();
        let id0 = store.insert_blob(Bytes::new()).map_err(|e| e.to_string())?;
        let id1 = store
            .insert_blob(Bytes::from_static(b"x"))
            .map_err(|e| e.to_string())?;
        assert_eq!(id0.get(), 0);
        assert_eq!(id1.get(), 1);
        Ok(())
    }

    // =========================================================================
    // Additional coverage: validate_* functions
    // =========================================================================

    #[test]
    fn validate_list_len_rejects_over_max() -> Result<(), String> {
        let too_many = MAX_LIST_ITEMS_PER_VALUE + 1;
        match super::validate_list_len(too_many) {
            Err(CoreError::ResourceLimitExceeded {
                resource: "list_items",
            }) => Ok(()),
            other => Err(format!("expected error, got {:?}", other)),
        }
    }

    #[test]
    fn validate_symbol_len_rejects_over_max() -> Result<(), String> {
        let too_long = MAX_SYMBOL_BYTES_PER_VALUE + 1;
        match super::validate_symbol_len(too_long) {
            Err(CoreError::ResourceLimitExceeded {
                resource: "symbol_bytes",
            }) => Ok(()),
            other => Err(format!("expected error, got {:?}", other)),
        }
    }

    #[test]
    fn validate_blob_len_rejects_over_max() -> Result<(), String> {
        let too_big = MAX_BLOB_BYTES_PER_VALUE + 1;
        match super::validate_blob_len(too_big) {
            Err(CoreError::ResourceLimitExceeded {
                resource: "blob_bytes",
            }) => Ok(()),
            other => Err(format!("expected error, got {:?}", other)),
        }
    }

    #[test]
    fn validate_object_len_rejects_over_max() -> Result<(), String> {
        let too_many = MAX_OBJECT_FIELDS_PER_VALUE + 1;
        match super::validate_object_len(too_many) {
            Err(CoreError::ResourceLimitExceeded {
                resource: "object_fields",
            }) => Ok(()),
            other => Err(format!("expected error, got {:?}", other)),
        }
    }

    // =========================================================================
    // Additional coverage: next_*_id overflow
    // =========================================================================

    #[test]
    fn next_symbol_id_overflow_returns_error() -> Result<(), String> {
        match super::next_symbol_id(u32::MAX as usize + 1) {
            Err(CoreError::ResourceLimitExceeded {
                resource: "symbols",
            }) => Ok(()),
            other => Err(format!("expected error, got {:?}", other)),
        }
    }

    #[test]
    fn next_list_id_overflow_returns_error() -> Result<(), String> {
        match super::next_list_id(u32::MAX as usize + 1) {
            Err(CoreError::ResourceLimitExceeded { resource: "lists" }) => Ok(()),
            other => Err(format!("expected error, got {:?}", other)),
        }
    }

    #[test]
    fn next_object_id_overflow_returns_error() -> Result<(), String> {
        match super::next_object_id(u32::MAX as usize + 1) {
            Err(CoreError::ResourceLimitExceeded {
                resource: "objects",
            }) => Ok(()),
            other => Err(format!("expected error, got {:?}", other)),
        }
    }

    #[test]
    fn next_blob_id_returns_valid_id() -> Result<(), String> {
        match super::next_blob_id(100) {
            Ok(id) => {
                if id.get() == 100 {
                    Ok(())
                } else {
                    Err(format!("expected 100, got {}", id.get()))
                }
            }
            other => Err(format!("expected Ok(BlobId(100)), got {:?}", other)),
        }
    }

    // =========================================================================
    // Additional coverage: symbol_index, list_index, object_index, blob_index
    // =========================================================================

    #[test]
    fn symbol_index_converts_id_to_usize() -> Result<(), String> {
        let id = SymbolId::new(42);
        let idx = super::symbol_index(id).map_err(|e| format!("{:?}", e))?;
        if idx == 42 {
            Ok(())
        } else {
            Err(format!("expected 42, got {}", idx))
        }
    }

    #[test]
    fn list_index_converts_id_to_usize() -> Result<(), String> {
        let id = ListId::new(42);
        let idx = super::list_index(id).map_err(|e| format!("{:?}", e))?;
        if idx == 42 {
            Ok(())
        } else {
            Err(format!("expected 42, got {}", idx))
        }
    }

    #[test]
    fn object_index_converts_id_to_usize() -> Result<(), String> {
        let id = ObjectId::new(42);
        let idx = super::object_index(id).map_err(|e| format!("{:?}", e))?;
        if idx == 42 {
            Ok(())
        } else {
            Err(format!("expected 42, got {}", idx))
        }
    }

    #[test]
    fn blob_index_converts_id_to_usize() -> Result<(), String> {
        let id = BlobId::new(42);
        let idx = super::blob_index(id).map_err(|e| format!("{:?}", e))?;
        if idx == 42 {
            Ok(())
        } else {
            Err(format!("expected 42, got {}", idx))
        }
    }

    #[test]
    fn symbol_index_accepts_valid_id() -> Result<(), String> {
        let id = SymbolId::new(100);
        let idx = super::symbol_index(id).map_err(|e| format!("{:?}", e))?;
        if idx == 100 {
            Ok(())
        } else {
            Err(format!("expected 100, got {}", idx))
        }
    }

    // =========================================================================
    // Additional coverage: Default for ValueStore
    // =========================================================================

    #[test]
    fn value_store_default_has_zero_arena_entries() -> Result<(), String> {
        let store: ValueStore = Default::default();
        assert_eq!(store.total_arena_count(), 0);
        assert_eq!(store.max_arena_entries(), 0);
        Ok(())
    }
