#![forbid(unsafe_code)]
//! Object and list construction helpers.

use crate::errors::EngineError;
use crate::ids::{ListId, ObjectId, SlotIdx, SymbolId};
use crate::value::{SlotValue, Taint, join_taint};
use crate::value_store::{ObjectField, ValueStore};

/// Reads object fields from frame slots into a pre-allocated vector.
fn read_object_fields(
    run: &crate::RunFrame,
    fields: &[(SymbolId, SlotIdx)],
) -> Result<Vec<ObjectField>, EngineError> {
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(fields.len())
        .map_err(|_| EngineError::AllocationFailed)?;
    append_object_fields(run, fields, &mut entries)?;
    Ok(entries)
}

fn append_object_fields(
    run: &crate::RunFrame,
    fields: &[(SymbolId, SlotIdx)],
    entries: &mut Vec<ObjectField>,
) -> Result<(), EngineError> {
    let mut index = 0usize;
    while index < fields.len() {
        let (key, slot) = fields
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "build_object field index checked by loop bound".into(),
            })?;
        entries.push(ObjectField {
            key: *key,
            value: *run.read_slot(*slot)?,
            taint: Taint::Clean,
        });
        index = next_object_field_index(index)?;
    }
    Ok(())
}

fn next_object_field_index(index: usize) -> Result<usize, EngineError> {
    index
        .checked_add(1)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "build_object field index overflow".into(),
        })
}

/// Constructs an object handle from field pairs read from frame slots.
pub fn build_object(
    store: &mut ValueStore,
    run: &crate::RunFrame,
    fields: &[(SymbolId, SlotIdx)],
) -> Result<ObjectId, EngineError> {
    let entries = read_object_fields(run, fields)?;
    store.insert_object(entries.into_boxed_slice())
}

/// Constructs an object handle and joins taint from all field source slots.
pub(crate) fn build_object_with_taint(
    store: &mut ValueStore,
    run: &crate::RunFrame,
    fields: &[(SymbolId, SlotIdx)],
) -> Result<(ObjectId, Taint), EngineError> {
    let (entries, accumulated_taint) = read_object_fields_with_taint(run, fields)?;
    let handle = store.insert_object(entries.into_boxed_slice())?;
    Ok((handle, accumulated_taint))
}

fn read_object_fields_with_taint(
    run: &crate::RunFrame,
    fields: &[(SymbolId, SlotIdx)],
) -> Result<(Vec<ObjectField>, Taint), EngineError> {
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(fields.len())
        .map_err(|_| EngineError::AllocationFailed)?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < fields.len() {
        let (key, slot) = fields
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "build_object field index checked by loop bound".into(),
            })?;
        let (entry, slot_taint) = read_object_field_with_taint(run, *key, *slot)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        entries.push(entry);
        index = next_object_field_index(index)?;
    }
    Ok((entries, accumulated_taint))
}
fn read_object_field_with_taint(
    run: &crate::RunFrame,
    key: SymbolId,
    slot: SlotIdx,
) -> Result<(ObjectField, Taint), EngineError> {
    let value = *run.read_slot(slot)?;
    let slot_taint = run.read_taint(slot)?;
    Ok((
        ObjectField {
            key,
            value,
            taint: slot_taint,
        },
        slot_taint,
    ))
}

/// Reads list items from frame slots into a pre-allocated vector.
fn read_list_items(
    run: &crate::RunFrame,
    items: &[SlotIdx],
) -> Result<Vec<SlotValue>, EngineError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(items.len())
        .map_err(|_| EngineError::AllocationFailed)?;
    let mut index = 0usize;
    while index < items.len() {
        let slot = items
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "build_list item index checked by loop bound".into(),
            })?;
        values.push(*run.read_slot(*slot)?);
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "build_list item index overflow".into(),
            })?;
    }
    Ok(values)
}

/// Constructs a list handle from slot values read from the frame.
pub fn build_list(
    store: &mut ValueStore,
    run: &crate::RunFrame,
    items: &[SlotIdx],
) -> Result<ListId, EngineError> {
    let values = read_list_items(run, items)?;
    store.insert_list(values.into_boxed_slice())
}

/// Constructs a list handle and joins taint from all item source slots.
pub(crate) fn build_list_with_taint(
    store: &mut ValueStore,
    run: &crate::RunFrame,
    items: &[SlotIdx],
) -> Result<(ListId, Taint), EngineError> {
    let (values, taints, accumulated_taint) = read_list_items_with_taint(run, items)?;
    let handle =
        store.insert_list_with_taint(values.into_boxed_slice(), taints.into_boxed_slice())?;
    Ok((handle, accumulated_taint))
}

fn read_list_items_with_taint(
    run: &crate::RunFrame,
    items: &[SlotIdx],
) -> Result<(Vec<SlotValue>, Vec<Taint>, Taint), EngineError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(items.len())
        .map_err(|_| EngineError::AllocationFailed)?;
    let mut taints = Vec::new();
    taints
        .try_reserve_exact(items.len())
        .map_err(|_| EngineError::AllocationFailed)?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < items.len() {
        let slot = list_item_slot(items, index)?;
        let (value, slot_taint) = read_list_item_with_taint(run, slot)?;
        values.push(value);
        taints.push(slot_taint);
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        index = next_list_item_index(index)?;
    }
    Ok((values, taints, accumulated_taint))
}
fn list_item_slot(items: &[SlotIdx], index: usize) -> Result<SlotIdx, EngineError> {
    items
        .get(index)
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "build_list item index checked by loop bound".into(),
        })
}
fn read_list_item_with_taint(
    run: &crate::RunFrame,
    slot: SlotIdx,
) -> Result<(SlotValue, Taint), EngineError> {
    let value = *run.read_slot(slot)?;
    let slot_taint = run.read_taint(slot)?;
    Ok((value, slot_taint))
}

fn next_list_item_index(index: usize) -> Result<usize, EngineError> {
    index
        .checked_add(1)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "build_list item index overflow".into(),
        })
}

#[cfg(test)]
#[path = "object_list/tests.rs"]
mod tests;
