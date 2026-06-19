//! Value store ID generation helpers.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};

#[allow(clippy::as_conversions)]
pub(super) fn checked_len_to_u64(len: usize) -> u64 {
    // Lossless on all Rust targets: usize is 32-bit or 64-bit.
    len as u64
}

pub(super) fn next_symbol_id(len: usize) -> CoreResult<SymbolId> {
    u32::try_from(len)
        .map(SymbolId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded {
            resource: "symbols",
        })
}

pub(super) fn next_list_id(len: usize) -> CoreResult<ListId> {
    u32::try_from(len)
        .map(ListId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded { resource: "lists" })
}

pub(super) fn next_object_id(len: usize) -> CoreResult<ObjectId> {
    u32::try_from(len)
        .map(ObjectId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded {
            resource: "objects",
        })
}

pub(super) fn next_blob_id(len: usize) -> CoreResult<BlobId> {
    u64::try_from(len)
        .map(BlobId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded { resource: "blobs" })
}

pub(super) fn symbol_index(id: SymbolId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::SymbolOutOfBounds { symbol: id })
}

pub(super) fn list_index(id: ListId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::ListOutOfBounds { list: id })
}

pub(super) fn object_index(id: ObjectId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::ObjectOutOfBounds { object: id })
}

pub(super) fn blob_index(id: BlobId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::BlobOutOfBounds { blob: id })
}
