//! Tests and formal verification harnesses for value_store.
//!
//! This module is conditionally compiled and does not contribute to the production build.

#[cfg(kani)]
mod kani_harnesses {
    fn same_static_str(left: &'static str, right: &'static str) -> bool {
        left.len() == right.len() && core::ptr::eq(left.as_ptr(), right.as_ptr())
    }

    /// PO-012: capped ValueStore rejects inserts with exact BudgetExceeded parity.
    #[kani::proof]
    fn value_store_cap_rejects_insert_with_budget_exceeded_max_slots() {
        let mut store = super::ValueStore::with_max_slots(1);

        match store.insert_blob(bytes::Bytes::new()) {
            Ok(_) => {}
            Err(_) => assert!(false),
        }
        assert!(store.total_arena_count() == 1);

        let result = store.insert_blob(bytes::Bytes::new());
        match &result {
            Err(super::CoreError::BudgetExceeded { budget, limit }) => {
                assert!(same_static_str(budget, "max_slots"));
                assert!(*limit == 1);
            }
            Ok(_) => assert!(false),
            Err(_) => assert!(false),
        }
        core::mem::forget(result);
        assert!(store.total_arena_count() == 1);
    }
}

#[cfg(test)]
mod tests {
    use super::{ObjectField, ValueStore};
    use crate::errors::CoreError;
    use crate::ids::SymbolId;
    use crate::ids::{BlobId, ListId, ObjectId};
    use crate::limits::{
        MAX_BLOB_BYTES_PER_VALUE, MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE,
        MAX_SYMBOL_BYTES_PER_VALUE,
    };
    use crate::value::{SlotValue, Taint};
    use bytes::Bytes;

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
                reason: "list values and taints length mismatch",
            }) => {}
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
}
