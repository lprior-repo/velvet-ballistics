use super::super::{ObjectField, ValueStore};
use crate::{SlotValue, Taint};
use serde::Serialize;

/// Serialize-only borrowed v1 checkpoint; field order matches the owned DTO.
#[derive(Debug, Serialize)]
pub struct ValueStoreCheckpointViewV1<'a> {
    symbols: &'a [Box<str>],
    lists: &'a [Box<[SlotValue]>],
    list_taints: &'a [Box<[Taint]>],
    objects: &'a [Box<[ObjectField]>],
    blobs: &'a [bytes::Bytes],
    max_arena_entries: u64,
}

impl ValueStore {
    /// Borrows every arena in identifier order without copying their contents.
    #[must_use]
    pub fn checkpoint_view_v1(&self) -> ValueStoreCheckpointViewV1<'_> {
        ValueStoreCheckpointViewV1 {
            symbols: &self.symbols,
            lists: &self.lists,
            list_taints: &self.list_taints,
            objects: &self.objects,
            blobs: &self.blobs,
            max_arena_entries: self.max_arena_entries,
        }
    }
}
