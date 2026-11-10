use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::value::{SlotValue, Taint};
use bytes::Bytes;

use super::ValueStore;
use super::blobs::blob_index;
use super::lists::list_index;
use super::objects::{ObjectField, object_index};
use super::symbols::symbol_index;

impl ValueStore {
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
}
