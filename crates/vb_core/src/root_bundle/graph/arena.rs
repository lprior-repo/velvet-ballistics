//! Dense arena index layout for owned root bundle graph validation.
//!
//! Maps typed arena node IDs to dense indices for O(1) scratch access
//! during graph traversal. Layout is computed once from the checkpoint
//! and used for all graph validation phases.

use crate::errors::CoreError;
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::root_bundle::errors::{ArenaNode, OwnedRootBundleError};
use crate::value::SlotValue;
use crate::value_store::ValueStoreCheckpointV1;

/// Dense index layout for the arena.
pub(in crate::root_bundle) struct Layout {
    symbol_count: u32,
    list_count: u32,
    object_count: u32,
    blob_count: u64,
    list_offset: usize,
    object_offset: usize,
    blob_offset: usize,
    total: usize,
}

/// Typed arena node counts extracted from checkpoint.
struct Counts {
    symbols: u32,
    lists: u32,
    objects: u32,
    blobs: u64,
}

/// Computed region offsets for arena layout.
struct Offsets {
    list: usize,
    object: usize,
    blob: usize,
    total: usize,
}

impl Layout {
    /// Compute the dense index layout from a checkpoint.
    pub(in crate::root_bundle) fn new(
        checkpoint: &ValueStoreCheckpointV1,
    ) -> Result<Self, OwnedRootBundleError> {
        let counts = convert_counts(checkpoint)?;
        Self::from_counts(counts)
    }

    /// Build layout from already-converted counts.
    fn from_counts(counts: Counts) -> Result<Self, OwnedRootBundleError> {
        let offsets = compute_offsets(&counts)?;
        Ok(Self {
            symbol_count: counts.symbols,
            list_count: counts.lists,
            object_count: counts.objects,
            blob_count: counts.blobs,
            list_offset: offsets.list,
            object_offset: offsets.object,
            blob_offset: offsets.blob,
            total: offsets.total,
        })
    }

    /// Total number of arena entries.
    pub(in crate::root_bundle) fn len(&self) -> usize {
        self.total
    }

    /// Map a typed arena node to its dense index.
    pub(in crate::root_bundle) fn index(
        &self,
        node: ArenaNode,
    ) -> Result<usize, OwnedRootBundleError> {
        match node {
            ArenaNode::Symbol(id) => at(0, id.get(), self.symbol_count),
            ArenaNode::List(id) => at(self.list_offset, id.get(), self.list_count),
            ArenaNode::Object(id) => at(self.object_offset, id.get(), self.object_count),
            ArenaNode::Blob(id) => checked_index(self.blob_offset, id.get(), self.blob_count),
        }
    }

    /// Map a dense index back to its typed arena node.
    pub(in crate::root_bundle) fn node(
        &self,
        index: usize,
    ) -> Result<ArenaNode, OwnedRootBundleError> {
        node_from_index(self, index)
    }
}

/// Convert checkpoint lengths to typed arena counts.
fn convert_counts(checkpoint: &ValueStoreCheckpointV1) -> Result<Counts, OwnedRootBundleError> {
    let symbols = u32::try_from(checkpoint.symbols.len()).map_err(|_| invariant())?;
    let lists = u32::try_from(checkpoint.lists.len()).map_err(|_| invariant())?;
    let objects = u32::try_from(checkpoint.objects.len()).map_err(|_| invariant())?;
    let blobs = u64::try_from(checkpoint.blobs.len()).map_err(|_| invariant())?;
    Ok(Counts {
        symbols,
        lists,
        objects,
        blobs,
    })
}

/// Compute region offsets from arena counts.
fn compute_offsets(counts: &Counts) -> Result<Offsets, OwnedRootBundleError> {
    let list = usize::try_from(counts.symbols).map_err(|_| invariant())?;
    let list_count = usize::try_from(counts.lists).map_err(|_| invariant())?;
    let object = list
        .checked_add(list_count)
        .ok_or(OwnedRootBundleError::ArenaCountOverflow)?;
    let object_count = usize::try_from(counts.objects).map_err(|_| invariant())?;
    let blob = object
        .checked_add(object_count)
        .ok_or(OwnedRootBundleError::ArenaCountOverflow)?;
    let blob_count = usize::try_from(counts.blobs).map_err(|_| invariant())?;
    let total = blob
        .checked_add(blob_count)
        .ok_or(OwnedRootBundleError::ArenaCountOverflow)?;
    Ok(Offsets {
        list,
        object,
        blob,
        total,
    })
}

/// Validate ID is within count and compute dense index.
fn checked_index(offset: usize, id: u64, count: u64) -> Result<usize, OwnedRootBundleError> {
    if id >= count {
        return Err(invariant());
    }
    let id_usize = usize::try_from(id).map_err(|_| invariant())?;
    offset
        .checked_add(id_usize)
        .ok_or(OwnedRootBundleError::ArenaCountOverflow)
}

/// Determine arena node from dense index.
fn node_from_index(layout: &Layout, index: usize) -> Result<ArenaNode, OwnedRootBundleError> {
    if index >= layout.total {
        return Err(invariant());
    }
    if index < layout.list_offset {
        return node_symbol(index);
    }
    if index < layout.object_offset {
        return node_list(relative(index, layout.list_offset)?);
    }
    if index < layout.blob_offset {
        return node_object(relative(index, layout.object_offset)?);
    }
    node_blob(relative(index, layout.blob_offset)?)
}

/// Construct Symbol arena node from index.
fn node_symbol(index: usize) -> Result<ArenaNode, OwnedRootBundleError> {
    let id = u32::try_from(index).map_err(|_| invariant())?;
    Ok(ArenaNode::Symbol(SymbolId::new(id)))
}

/// Construct List arena node from offset.
fn node_list(offset: usize) -> Result<ArenaNode, OwnedRootBundleError> {
    let id = u32::try_from(offset).map_err(|_| invariant())?;
    Ok(ArenaNode::List(ListId::new(id)))
}

/// Construct Object arena node from offset.
fn node_object(offset: usize) -> Result<ArenaNode, OwnedRootBundleError> {
    let id = u32::try_from(offset).map_err(|_| invariant())?;
    Ok(ArenaNode::Object(ObjectId::new(id)))
}

/// Construct Blob arena node from offset.
fn node_blob(offset: usize) -> Result<ArenaNode, OwnedRootBundleError> {
    let id = u64::try_from(offset).map_err(|_| invariant())?;
    Ok(ArenaNode::Blob(BlobId::new(id)))
}

/// Map a slot value to its arena node, if any.
pub(in crate::root_bundle) fn node_of(value: SlotValue) -> Option<ArenaNode> {
    match value {
        SlotValue::Null => None,
        SlotValue::Bool(_) => None,
        SlotValue::I64(_) => None,
        SlotValue::F64(_) => None,
        SlotValue::Symbol(id) => Some(ArenaNode::Symbol(id)),
        SlotValue::List(id) => Some(ArenaNode::List(id)),
        SlotValue::Object(id) => Some(ArenaNode::Object(id)),
        SlotValue::Blob(id) => Some(ArenaNode::Blob(id)),
    }
}

/// Create an internal invariant violation error.
pub(in crate::root_bundle) fn invariant() -> OwnedRootBundleError {
    OwnedRootBundleError::Checkpoint(CoreError::InternalInvariantViolation {
        reason: "owned_root_graph_shape",
    })
}

/// Fallibly allocate a Vec with exact capacity, then resize.
pub(in crate::root_bundle) fn reserve<T: Clone>(
    len: usize,
    value: T,
) -> Result<Vec<T>, OwnedRootBundleError> {
    let mut vec = Vec::new();
    vec.try_reserve_exact(len)
        .map_err(|_| OwnedRootBundleError::Checkpoint(CoreError::AllocationFailed))?;
    vec.resize(len, value);
    Ok(vec)
}
fn at(offset: usize, id: u32, count: u32) -> Result<usize, OwnedRootBundleError> {
    checked_index(offset, u64::from(id), u64::from(count))
}

fn relative(index: usize, offset: usize) -> Result<usize, OwnedRootBundleError> {
    index.checked_sub(offset).ok_or_else(invariant)
}
