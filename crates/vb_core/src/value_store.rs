#![forbid(unsafe_code)]
//! Cold arenas backing handle-only runtime slot values.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};

use crate::value::{SlotValue, Taint};
use bytes::Bytes;
use indexmap::IndexMap;

pub mod transport;
pub use transport::ValueStoreCheckpointViewV1;
pub use transport::{
    CheckpointShape, CheckpointTransportBudget, CheckpointTransportError, CheckpointWireCursor,
    TransportUsage, preflight_checkpoint,
};

mod checkpoint;
pub use checkpoint::ValueStoreCheckpointV1;
mod blobs;
mod lists;
mod objects;
mod symbols;
use blobs::{blob_index, next_blob_id, validate_blob_len};
pub(crate) use checkpoint::{validate_checkpoint, validate_value_reference};
use lists::{list_index, next_list_id, validate_list_len};
pub use objects::ObjectField;
use objects::{next_object_id, object_index, validate_object_len};
use symbols::{next_symbol_id, symbol_index, validate_symbol_len};

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

    /// Deep byte size of a value: payload bytes of every scalar and
    /// every arena entry reachable from it, traversing list and object
    /// handles instead of measuring the handles themselves. Scalars
    /// count as Null 0, Bool 1, I64 8, F64 8; a symbol counts its
    /// string bytes, a blob its payload bytes, and an object field its
    /// key bytes plus its value. Each handle is counted once, so a
    /// handle shared by several parents contributes once and a handle
    /// cycle terminates; traversal work is therefore bounded by the
    /// reachable arena contents rather than by the expanded tree,
    /// which sharing could blow up exponentially. An unresolvable
    /// handle or an overflowing sum is an error, never a silent
    /// undercount.
    pub fn deep_size(&self, value: &SlotValue) -> CoreResult<u64> {
        let mut total: u64 = 0;
        let mut visited_symbols: std::collections::HashSet<SymbolId> =
            std::collections::HashSet::new();
        let mut visited_lists: std::collections::HashSet<ListId> = std::collections::HashSet::new();
        let mut visited_objects: std::collections::HashSet<ObjectId> =
            std::collections::HashSet::new();
        let mut visited_blobs: std::collections::HashSet<BlobId> = std::collections::HashSet::new();
        let mut stack: Vec<SlotValue> = vec![*value];
        while let Some(current) = stack.pop() {
            match current {
                SlotValue::Null => {}
                SlotValue::Bool(_) => {
                    total = add_size(total, 1)?;
                }
                SlotValue::I64(_) | SlotValue::F64(_) => {
                    total = add_size(total, 8)?;
                }
                SlotValue::Symbol(id) => {
                    if visited_symbols.insert(id) {
                        let len = self.symbol(id)?.len();
                        total = add_size(total, len_to_u64(len)?)?;
                    }
                }
                SlotValue::Blob(id) => {
                    if visited_blobs.insert(id) {
                        let len = self.blob(id)?.len();
                        total = add_size(total, len_to_u64(len)?)?;
                    }
                }
                SlotValue::List(id) => {
                    if visited_lists.insert(id) {
                        for item in self.list(id)? {
                            stack.push(*item);
                        }
                    }
                }
                SlotValue::Object(id) => {
                    if visited_objects.insert(id) {
                        for field in self.object(id)? {
                            stack.push(SlotValue::Symbol(field.key));
                            stack.push(field.value);
                        }
                    }
                }
            }
        }
        Ok(total)
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

fn checked_len_to_u64(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
}

fn add_size(total: u64, addend: u64) -> CoreResult<u64> {
    total
        .checked_add(addend)
        .ok_or(CoreError::ResourceLimitExceeded {
            resource: "result_bytes",
        })
}

fn len_to_u64(len: usize) -> CoreResult<u64> {
    u64::try_from(len).map_err(|_| CoreError::ResourceLimitExceeded {
        resource: "result_bytes",
    })
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
            Err(_) => kani::assert(false, "insert into empty store must succeed"),
        }
        assert!(store.total_arena_count() == 1);

        let result = store.insert_blob(bytes::Bytes::new());
        match &result {
            Err(super::CoreError::BudgetExceeded { budget, limit }) => {
                assert!(same_static_str(budget, "max_slots"));
                assert!(*limit == 1);
            }
            Ok(_) => kani::assert(false, "second insert into capped store must fail"),
            Err(_) => kani::assert(false, "only BudgetExceeded valid here"),
        }
        core::mem::forget(result);
        assert!(store.total_arena_count() == 1);
    }
}

#[cfg(test)]
mod extended_tests;

#[cfg(any(test, kani))]
include!("value_store/tests_and_verification.rs");

#[cfg(test)]
#[path = "value_store/tests.rs"]
mod tests;
