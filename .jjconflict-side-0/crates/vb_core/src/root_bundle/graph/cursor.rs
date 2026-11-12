//! Graph edge cursor for owned root bundle validation.
//!
//! Provides O(1) edge access by cursor index without materializing
//! the full edge list. Used by graph validation phases.

use super::arena;
use crate::ids::ListId;
use crate::root_bundle::errors::OwnedRootBundleError;
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStoreCheckpointV1;

use super::ArenaNode;

mod object;

/// A single graph edge with its value and taint.
pub(in crate::root_bundle) struct Edge {
    /// The value at this edge.
    pub value: SlotValue,
    /// The taint at this edge.
    pub taint: Taint,
}

/// Get the edge at the given cursor position for a node.
///
/// Returns None if the cursor is past the end of the node's children.
pub(in crate::root_bundle) fn edge(
    cp: &ValueStoreCheckpointV1,
    node: ArenaNode,
    cursor: usize,
) -> Result<Option<Edge>, OwnedRootBundleError> {
    match node {
        ArenaNode::List(id) => list_edge(cp, id, cursor),
        ArenaNode::Object(id) => object::object_edge(cp, id, cursor),
        ArenaNode::Symbol(_) | ArenaNode::Blob(_) => Ok(None),
    }
}

fn list_taint(
    cp: &ValueStoreCheckpointV1,
    list_idx: usize,
    cursor: usize,
) -> Result<Taint, OwnedRootBundleError> {
    let taints = cp.list_taints.get(list_idx).ok_or_else(arena::invariant)?;
    let taint = taints.get(cursor).ok_or_else(arena::invariant)?;
    Ok(*taint)
}

fn list_edge(
    cp: &ValueStoreCheckpointV1,
    id: ListId,
    cursor: usize,
) -> Result<Option<Edge>, OwnedRootBundleError> {
    let id_val = id.get();
    let list_idx = usize::try_from(id_val).map_err(|_| arena::invariant())?;
    if list_idx >= cp.lists.len() {
        return Err(arena::invariant());
    }
    let list = cp.lists.get(list_idx).ok_or_else(arena::invariant)?;
    if cursor >= list.len() {
        return Ok(None);
    }
    let value = *list.get(cursor).ok_or_else(arena::invariant)?;
    let taint = list_taint(cp, list_idx, cursor)?;
    Ok(Some(Edge { value, taint }))
}
