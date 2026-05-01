//! Cold arenas backing handle-only runtime slot values.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::limits::{MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE};
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
        let id = next_symbol_id(self.symbols.len())?;
        self.symbols.push(value.into());
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
        let id = next_blob_id(self.blobs.len())?;
        self.blobs.push(bytes.into());
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
    use crate::limits::{MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE};
    use crate::value::SlotValue;

    #[test]
    fn insert_list_rejects_payload_over_hard_bound() -> Result<(), String> {
        let mut store = ValueStore::new();
        let values =
            vec![SlotValue::Null; MAX_LIST_ITEMS_PER_VALUE.saturating_add(1)].into_boxed_slice();

        match store.insert_list(values) {
            Err(CoreError::ResourceLimitExceeded { resource }) if resource == "list_items" => {
                Ok(())
            }
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
            Err(CoreError::ResourceLimitExceeded { resource }) if resource == "object_fields" => {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }
}
