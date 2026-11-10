#![forbid(unsafe_code)]
//! Cold arenas backing handle-only runtime slot values.

use crate::errors::{CoreError, CoreResult};
use crate::ids::SymbolId;

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
mod deep_size;
mod insert;
mod lists;
mod objects;
mod resolve;
mod symbols;
pub(crate) use blobs::validate_blob_len;
pub(crate) use checkpoint::{validate_checkpoint, validate_value_reference};
pub(crate) use lists::validate_list_len;
pub use objects::ObjectField;
pub(crate) use objects::validate_object_len;
pub(crate) use symbols::validate_symbol_len;
#[cfg(test)]
pub(crate) use blobs::{blob_index, next_blob_id};
#[cfg(test)]
pub(crate) use lists::{list_index, next_list_id};
#[cfg(test)]
pub(crate) use objects::{next_object_id, object_index};
#[cfg(test)]
pub(crate) use symbols::{next_symbol_id, symbol_index};

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

fn checked_len_to_u64(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
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
