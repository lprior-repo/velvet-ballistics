#![forbid(unsafe_code)]
//! Cold arenas backing handle-only runtime slot values.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::limits::{
    MAX_BLOB_BYTES_PER_VALUE, MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE,
    MAX_SYMBOL_BYTES_PER_VALUE,
};
use crate::value::{SlotValue, Taint};
use bytes::Bytes;
use indexmap::IndexMap;

/// Deterministic object field stored in insertion order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectField {
    /// Interned field name.
    pub key: SymbolId,
    /// Handle-only field value.
    pub value: SlotValue,
    /// Taint level of the field value.
    pub taint: Taint,
}

impl ObjectField {
    /// Creates a clean-tainted object field.
    #[must_use]
    pub const fn clean(key: SymbolId, value: SlotValue) -> Self {
        Self {
            key,
            value,
            taint: Taint::Clean,
        }
    }

    /// Creates an object field with explicit taint.
    #[must_use]
    pub const fn with_taint(key: SymbolId, value: SlotValue, taint: Taint) -> Self {
        Self { key, value, taint }
    }
}

/// Cold value arenas for strings, lists, objects, and blobs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValueStore {
    symbols: Vec<Box<str>>,
    lists: Vec<Box<[SlotValue]>>,
    /// Per-item taint parallel to `lists`, same indexing.
    list_taints: Vec<Box<[Taint]>>,
    objects: Vec<Box<[ObjectField]>>,
    /// Secondary index for O(1) object field lookup, mirroring `objects`.
    object_field_index: Vec<IndexMap<SymbolId, SlotValue>>,
    /// Per-field taint parallel to `object_field_index`, keyed the same way.
    object_taint_index: Vec<IndexMap<SymbolId, Taint>>,
    blobs: Vec<Bytes>,
    /// Hard cap on total arena entries (sum of all arena lengths).
    max_arena_entries: u64,
}

impl ValueStore {
    /// Creates an empty cold value store with no arena cap.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            symbols: Vec::new(),
            lists: Vec::new(),
            list_taints: Vec::new(),
            objects: Vec::new(),
            object_field_index: Vec::new(),
            object_taint_index: Vec::new(),
            blobs: Vec::new(),
            max_arena_entries: 0,
        }
    }

    /// Creates a cold value store with a hard cap on total arena entries.
    #[must_use]
    pub fn with_max_slots(max_slots: u16) -> Self {
        Self {
            symbols: Vec::new(),
            lists: Vec::new(),
            list_taints: Vec::new(),
            objects: Vec::new(),
            object_field_index: Vec::new(),
            object_taint_index: Vec::new(),
            blobs: Vec::new(),
            max_arena_entries: u64::from(max_slots),
        }
    }

    /// Inserts an interned symbol and returns its deterministic insertion ID.
    pub fn insert_symbol(&mut self, value: impl Into<Box<str>>) -> CoreResult<SymbolId> {
        let value = value.into();
        validate_symbol_len(value.len())?;
        self.check_arena_cap()?;
        let id = next_symbol_id(self.symbols.len())?;
        self.symbols.push(value);
        Ok(id)
    }

    /// Inserts a list payload and returns its deterministic insertion ID.
    /// All items are stored with `Taint::Clean`.
    pub fn insert_list(&mut self, values: impl Into<Box<[SlotValue]>>) -> CoreResult<ListId> {
        let values = values.into();
        validate_list_len(values.len())?;
        self.check_arena_cap()?;
        let id = next_list_id(self.lists.len())?;
        let taints = vec![Taint::Clean; values.len()].into_boxed_slice();
        self.list_taints.push(taints);
        self.lists.push(values);
        Ok(id)
    }

    /// Inserts a list payload with per-item taint and returns its deterministic insertion ID.
    pub fn insert_list_with_taint(
        &mut self,
        values: Box<[SlotValue]>,
        taints: Box<[Taint]>,
    ) -> CoreResult<ListId> {
        validate_list_len(values.len())?;
        if taints.len() != values.len() {
            return Err(CoreError::InternalInvariantViolation {
                reason: "list values and taints length mismatch",
            });
        }
        self.check_arena_cap()?;
        let id = next_list_id(self.lists.len())?;
        self.list_taints.push(taints);
        self.lists.push(values);
        Ok(id)
    }

    /// Inserts object fields in caller-provided deterministic order.
    pub fn insert_object(&mut self, fields: impl Into<Box<[ObjectField]>>) -> CoreResult<ObjectId> {
        let fields = fields.into();
        validate_object_len(fields.len())?;
        self.check_arena_cap()?;
        let id = next_object_id(self.objects.len())?;
        let mut index = IndexMap::new();
        let mut taint_index = IndexMap::new();
        let mut field_pos = 0usize;
        while field_pos < fields.len() {
            let field = fields
                .get(field_pos)
                .ok_or(CoreError::InternalInvariantViolation {
                    reason: "object field index checked by loop bound",
                })?;
            index.entry(field.key).or_insert(field.value);
            taint_index.entry(field.key).or_insert(field.taint);
            field_pos = field_pos
                .checked_add(1)
                .ok_or(CoreError::InternalInvariantViolation {
                    reason: "object field index overflow",
                })?;
        }
        self.object_field_index.push(index);
        self.object_taint_index.push(taint_index);
        self.objects.push(fields);
        Ok(id)
    }

    /// Inserts a byte blob and returns its deterministic insertion ID.
    pub fn insert_blob(&mut self, bytes: impl Into<Bytes>) -> CoreResult<BlobId> {
        let bytes = bytes.into();
        validate_blob_len(bytes.len())?;
        self.check_arena_cap()?;
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

    /// Resolves one list element with its stored taint from a list arena handle.
    pub fn list_item_with_taint(&self, id: ListId, index: u32) -> CoreResult<(SlotValue, Taint)> {
        let item_index =
            usize::try_from(index).map_err(|_| CoreError::ListIndexOutOfBounds { index })?;
        let list_idx = list_index(id)?;
        let value = self
            .lists
            .get(list_idx)
            .ok_or(CoreError::ListOutOfBounds { list: id })?
            .get(item_index)
            .copied()
            .ok_or(CoreError::ListIndexOutOfBounds { index })?;
        let taint = self
            .list_taints
            .get(list_idx)
            .ok_or(CoreError::ListOutOfBounds { list: id })?
            .get(item_index)
            .copied()
            .ok_or(CoreError::ListIndexOutOfBounds { index })?;
        Ok((value, taint))
    }

    /// Resolves one object field from an object arena handle.
    pub fn object_field(&self, id: ObjectId, key: SymbolId) -> CoreResult<SlotValue> {
        let idx = object_index(id)?;
        let index = self
            .object_field_index
            .get(idx)
            .ok_or(CoreError::ObjectOutOfBounds { object: id })?;
        index
            .get(&key)
            .copied()
            .ok_or(CoreError::ObjectFieldNotFound { field: key })
    }

    /// Resolves one object field with its stored taint from an object arena handle.
    pub fn object_field_with_taint(
        &self,
        id: ObjectId,
        key: SymbolId,
    ) -> CoreResult<(SlotValue, Taint)> {
        let idx = object_index(id)?;
        let index = self
            .object_field_index
            .get(idx)
            .ok_or(CoreError::ObjectOutOfBounds { object: id })?;
        let value = index
            .get(&key)
            .copied()
            .ok_or(CoreError::ObjectFieldNotFound { field: key })?;
        let taint_index = self
            .object_taint_index
            .get(idx)
            .ok_or(CoreError::ObjectOutOfBounds { object: id })?;
        let taint = taint_index
            .get(&key)
            .copied()
            .ok_or(CoreError::ObjectFieldNotFound { field: key })?;
        Ok((value, taint))
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

    /// Total number of entries across all arenas.
    #[must_use]
    pub fn total_arena_count(&self) -> u64 {
        let mut total = 0u64;
        total = total.saturating_add(checked_len_to_u64(self.symbols.len()));
        total = total.saturating_add(checked_len_to_u64(self.lists.len()));
        total = total.saturating_add(checked_len_to_u64(self.objects.len()));
        total = total.saturating_add(checked_len_to_u64(self.blobs.len()));
        total
    }

    /// Returns the configured max arena entries (0 means uncapped).
    #[must_use]
    pub const fn max_arena_entries(&self) -> u64 {
        self.max_arena_entries
    }

    /// Checks whether a new arena entry would exceed the cap.
    fn check_arena_cap(&self) -> CoreResult<()> {
        if self.max_arena_entries == 0 {
            return Ok(());
        }
        let current = self.total_arena_count();
        if current >= self.max_arena_entries {
            return Err(CoreError::BudgetExceeded {
                budget: "max_slots",
                limit: self.max_arena_entries,
            });
        }
        Ok(())
    }
}

#[allow(clippy::as_conversions)]
fn checked_len_to_u64(len: usize) -> u64 {
    // Lossless on all Rust targets: usize is either 32-bit or 64-bit.
    // Both fit in u64, so this cast never overflows or truncates.
    len as u64
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
    usize::try_from(id.get()).map_err(|_| CoreError::BlobOutOfBounds { blob: id })
}

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
            Err(_) => kani::assert(false), }
        assert!(store.total_arena_count() == 1);

        let result = store.insert_blob(bytes::Bytes::new());
        match &result {
            Err(super::CoreError::BudgetExceeded { budget, limit }) => {
                kani::assert(same_static_str(budget, "max_slots"));
                kani::assert(*limit == 1, "kani harness assertion")
            }
            Ok(_) => kani::assert(false),
            Err(_) => assert!(false), }
        core::mem::forget(result);
        kani::assert(store.total_arena_count() == 1, "kani harness assertion")
    }
}

#[cfg(all(test, kani))]
mod extended_tests;

#[cfg(test)]
#[path = "value_store/tests.rs"]
mod tests;
