//! Cold arenas backing handle-only runtime slot values.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::limits::{
    MAX_BLOB_BYTES_PER_VALUE, MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE,
    MAX_SYMBOL_BYTES_PER_VALUE,
};
use crate::value::SlotValue;
use bytes::Bytes;

/// Deterministic object field stored in insertion order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectField {
    /// Interned field name.
    pub key: SymbolId,
    /// Handle-only field value.
    pub value: SlotValue,
}

/// Cold value arenas for strings, lists, objects, and blobs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValueStore {
    symbols: Vec<Box<str>>,
    lists: Vec<Box<[SlotValue]>>,
    objects: Vec<Box<[ObjectField]>>,
    blobs: Vec<Bytes>,
}

impl ValueStore {
    /// Creates an empty cold value store.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            symbols: Vec::new(),
            lists: Vec::new(),
            objects: Vec::new(),
            blobs: Vec::new(),
        }
    }

    /// Inserts an interned symbol and returns its deterministic insertion ID.
    pub fn insert_symbol(&mut self, value: impl Into<Box<str>>) -> CoreResult<SymbolId> {
        let value = value.into();
        validate_symbol_len(value.len())?;
        let id = next_symbol_id(self.symbols.len())?;
        self.symbols.push(value);
        Ok(id)
    }

    /// Inserts a list payload and returns its deterministic insertion ID.
    pub fn insert_list(&mut self, values: impl Into<Box<[SlotValue]>>) -> CoreResult<ListId> {
        let values = values.into();
        validate_list_len(values.len())?;
        let id = next_list_id(self.lists.len())?;
        self.lists.push(values);
        Ok(id)
    }

    /// Inserts object fields in caller-provided deterministic order.
    pub fn insert_object(&mut self, fields: impl Into<Box<[ObjectField]>>) -> CoreResult<ObjectId> {
        let fields = fields.into();
        validate_object_len(fields.len())?;
        let id = next_object_id(self.objects.len())?;
        self.objects.push(fields);
        Ok(id)
    }

    /// Inserts a byte blob and returns its deterministic insertion ID.
    pub fn insert_blob(&mut self, bytes: impl Into<Bytes>) -> CoreResult<BlobId> {
        let bytes = bytes.into();
        validate_blob_len(bytes.len())?;
        let id = next_blob_id(self.blobs.len())?;
        self.blobs.push(bytes);
        Ok(id)
    }

    /// Resolves a symbol handle to its stored string payload.
    pub fn symbol(&self, id: SymbolId) -> CoreResult<&str> {
        self.symbols
            .get(symbol_index(id)?)
            .map(Box::as_ref)
            .ok_or(CoreError::SymbolOutOfBounds { symbol: id })
    }

    /// Resolves a list handle to its stored slot-value slice.
    pub fn list(&self, id: ListId) -> CoreResult<&[SlotValue]> {
        self.lists
            .get(list_index(id)?)
            .map(Box::as_ref)
            .ok_or(CoreError::ListOutOfBounds { list: id })
    }

    /// Resolves an object handle to its deterministic field slice.
    pub fn object(&self, id: ObjectId) -> CoreResult<&[ObjectField]> {
        self.objects
            .get(object_index(id)?)
            .map(Box::as_ref)
            .ok_or(CoreError::ObjectOutOfBounds { object: id })
    }

    /// Resolves a blob handle to its stored byte payload.
    pub fn blob(&self, id: BlobId) -> CoreResult<&[u8]> {
        self.blobs
            .get(blob_index(id)?)
            .map(Bytes::as_ref)
            .ok_or(CoreError::BlobOutOfBounds { blob: id })
    }

    /// Resolves one list element from a list arena handle.
    pub fn list_item(&self, id: ListId, index: u32) -> CoreResult<SlotValue> {
        let item_index =
            usize::try_from(index).map_err(|_| CoreError::ListIndexOutOfBounds { index })?;
        self.list(id)?
            .get(item_index)
            .copied()
            .ok_or(CoreError::ListIndexOutOfBounds { index })
    }

    /// Resolves one object field from an object arena handle.
    pub fn object_field(&self, id: ObjectId, key: SymbolId) -> CoreResult<SlotValue> {
        let fields = self.object(id)?;
        let mut index = 0usize;
        while index < fields.len() {
            let field = fields
                .get(index)
                .ok_or(CoreError::InternalInvariantViolation {
                    reason: "object field index checked by loop bound",
                })?;
            if field.key == key {
                return Ok(field.value);
            }
            index = index
                .checked_add(1)
                .ok_or(CoreError::InternalInvariantViolation {
                    reason: "object field index overflow",
                })?;
        }
        Err(CoreError::ObjectFieldNotFound { field: key })
    }

    /// Number of stored symbols.
    #[must_use]
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// Number of stored lists.
    #[must_use]
    pub fn list_count(&self) -> usize {
        self.lists.len()
    }

    /// Number of stored objects.
    #[must_use]
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Number of stored blobs.
    #[must_use]
    pub fn blob_count(&self) -> usize {
        self.blobs.len()
    }
}

fn next_symbol_id(len: usize) -> CoreResult<SymbolId> {
    u32::try_from(len)
        .map(SymbolId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded {
            resource: "symbols",
        })
}

fn next_list_id(len: usize) -> CoreResult<ListId> {
    u32::try_from(len)
        .map(ListId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded { resource: "lists" })
}

fn next_object_id(len: usize) -> CoreResult<ObjectId> {
    u32::try_from(len)
        .map(ObjectId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded {
            resource: "objects",
        })
}

fn next_blob_id(len: usize) -> CoreResult<BlobId> {
    u64::try_from(len)
        .map(BlobId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded { resource: "blobs" })
}

fn validate_list_len(len: usize) -> CoreResult<()> {
    if len > MAX_LIST_ITEMS_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "list_items",
        })
    } else {
        Ok(())
    }
}

fn validate_symbol_len(len: usize) -> CoreResult<()> {
    if len > MAX_SYMBOL_BYTES_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "symbol_bytes",
        })
    } else {
        Ok(())
    }
}

fn validate_blob_len(len: usize) -> CoreResult<()> {
    if len > MAX_BLOB_BYTES_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "blob_bytes",
        })
    } else {
        Ok(())
    }
}

fn validate_object_len(len: usize) -> CoreResult<()> {
    if len > MAX_OBJECT_FIELDS_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "object_fields",
        })
    } else {
        Ok(())
    }
}

fn symbol_index(id: SymbolId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::SymbolOutOfBounds { symbol: id })
}

fn list_index(id: ListId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::ListOutOfBounds { list: id })
}

fn object_index(id: ObjectId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::ObjectOutOfBounds { object: id })
}

fn blob_index(id: BlobId) -> CoreResult<usize> {
    usize::try_from(id.as_u64()).map_err(|_| CoreError::BlobOutOfBounds { blob: id })
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
    use crate::value::SlotValue;
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
                    },
                    ObjectField {
                        key: dup_key,
                        value: SlotValue::I64(200),
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
    fn value_store_object_at_exact_max_fields_is_accepted() -> Result<(), String> {
        let mut store = ValueStore::new();
        let field = ObjectField {
            key: SymbolId::new(0),
            value: SlotValue::Null,
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
        if b0.as_u64() != 0 || b1.as_u64() != 1 {
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
                    },
                    ObjectField {
                        key: shared_key,
                        value: SlotValue::I64(2),
                    },
                    ObjectField {
                        key: unique_key,
                        value: SlotValue::I64(3),
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
}
