use super::CheckpointWireCursor as Cursor;
use super::{CheckpointTransportBudget as Budget, CheckpointTransportError as Error};
use crate::limits::{
    MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE, MAX_SYMBOL_BYTES_PER_VALUE,
};

/// Independently accepted checkpoint shape, not supplied by the hostile payload.
#[derive(Debug, Clone, Copy)]
pub struct CheckpointShape {
    /// Exact positive compiled arena cap.
    pub arena_entries: usize,
    /// Per-blob cap, already restricted to the existing core hard limit.
    pub blob_bytes: usize,
}

/// Checks every checkpoint container and scalar before owned deserialization.
/// Graph identity, references, cycle/depth and taint checks remain strong-validator work.
pub fn preflight_checkpoint(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
    shape: CheckpointShape,
) -> Result<(), Error> {
    if shape.arena_entries == 0
        || shape.arena_entries > usize::from(u16::MAX)
        || shape.blob_bytes > crate::limits::MAX_BLOB_BYTES_PER_VALUE
    {
        return Err(Error::InvalidLimit);
    }
    let symbols = symbols(cursor, budget, shape.arena_entries)?;
    let (lists, list_edges) = lists(cursor, budget, shape.arena_entries)?;
    taints(cursor, budget, lists)?;
    let objects = objects(cursor, budget, shape.arena_entries, list_edges)?;
    let blobs = blobs(cursor, budget, shape)?;
    let total = symbols
        .checked_add(lists)
        .and_then(|n| n.checked_add(objects))
        .and_then(|n| n.checked_add(blobs))
        .ok_or(Error::Overflow)?;
    if total > shape.arena_entries {
        return Err(Error::Bound("arena_total"));
    }
    let expected = u64::try_from(shape.arena_entries).map_err(|_| Error::Overflow)?;
    if cursor.unsigned(64)? != expected {
        return Err(Error::Bound("arena_cap"));
    }
    // Graph colors/key markers plus every possible bounded 64-frame DFS stack.
    budget.reserve(total, 256)?;
    budget.work(total.checked_mul(16).ok_or(Error::Overflow)?)?;
    budget.reserve(lists.checked_add(objects).ok_or(Error::Overflow)?, 4096)?;
    Ok(())
}

fn symbols(cursor: &mut Cursor<'_>, budget: &mut Budget, cap: usize) -> Result<usize, Error> {
    let count = cursor.length(cap, "symbols")?;
    budget.reserve(count, 64)?;
    budget.work(count)?;
    for _ in 0..count {
        let len = cursor.length(MAX_SYMBOL_BYTES_PER_VALUE, "symbol_bytes")?;
        budget.reserve(len, 4)?;
        budget.work(len)?;
        std::str::from_utf8(cursor.take(len)?).map_err(|_| Error::Malformed("symbol_utf8"))?;
    }
    Ok(count)
}

fn lists(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
    cap: usize,
) -> Result<(usize, usize), Error> {
    let count = cursor.length(cap, "lists")?;
    budget.reserve(count, 64)?;
    budget.work(count)?;
    let mut edges = 0_usize;
    for _ in 0..count {
        let len = cursor.length(MAX_LIST_ITEMS_PER_VALUE, "list_items")?;
        edges = edges.checked_add(len).ok_or(Error::Overflow)?;
        if edges > 1_000_000 {
            return Err(Error::Bound("edge_work"));
        }
        budget.reserve(len, 64)?;
        budget.work(len)?;
        for _ in 0..len {
            cursor.value()?;
        }
    }
    budget.work(edges.checked_mul(16).ok_or(Error::Overflow)?)?;
    Ok((count, edges))
}

fn taints(cursor: &mut Cursor<'_>, budget: &mut Budget, lists: usize) -> Result<(), Error> {
    let count = cursor.length(lists, "list_taints")?;
    if count != lists {
        return Err(Error::Bound("list_taints"));
    }
    budget.reserve(count, 64)?;
    budget.work(count)?;
    for _ in 0..count {
        let len = cursor.length(MAX_LIST_ITEMS_PER_VALUE, "item_taints")?;
        budget.reserve(len, 4)?;
        budget.work(len)?;
        for _ in 0..len {
            cursor.taint()?;
        }
    }
    Ok(())
}

fn objects(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
    cap: usize,
    list_edges: usize,
) -> Result<usize, Error> {
    let count = cursor.length(cap, "objects")?;
    budget.reserve(count, 64)?;
    budget.work(count)?;
    let mut edges = list_edges;
    for _ in 0..count {
        let len = cursor.length(MAX_OBJECT_FIELDS_PER_VALUE, "object_fields")?;
        edges = edges
            .checked_add(len.checked_mul(2).ok_or(Error::Overflow)?)
            .ok_or(Error::Overflow)?;
        if edges > 1_000_000 {
            return Err(Error::Bound("edge_work"));
        }
        budget.reserve(len, 256)?;
        budget.work(len.checked_mul(2).ok_or(Error::Overflow)?)?;
        for _ in 0..len {
            cursor.unsigned(32)?;
            cursor.value()?;
            cursor.taint()?;
        }
    }
    budget.work(
        edges
            .checked_sub(list_edges)
            .and_then(|n| n.checked_mul(16))
            .ok_or(Error::Overflow)?,
    )?;
    Ok(count)
}

fn blobs(
    cursor: &mut Cursor<'_>,
    budget: &mut Budget,
    shape: CheckpointShape,
) -> Result<usize, Error> {
    let count = cursor.length(shape.arena_entries, "blobs")?;
    budget.reserve(count, 128)?;
    budget.work(count)?;
    for _ in 0..count {
        let len = cursor.length(shape.blob_bytes, "blob_bytes")?;
        budget.reserve(len, 4)?;
        budget.work(len)?;
        cursor.take(len)?;
    }
    Ok(count)
}
