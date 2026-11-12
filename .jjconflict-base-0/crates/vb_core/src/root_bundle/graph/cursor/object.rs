//! Object edge cursor helpers for root bundle graph validation.
//!
//! Provides field lookup and field-to-Edge conversion for object nodes.
//! Object edges alternate key/value, so cursor indices are halved to
//! compute field indices.

use super::super::arena;
use super::Edge;
use crate::ids::ObjectId;
use crate::root_bundle::errors::OwnedRootBundleError;
use crate::value::{SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStoreCheckpointV1};

/// Compute the field index from an object edge cursor.
/// Object edges alternate key/value, so field index is cursor / 2.
pub(in crate::root_bundle) fn object_field_index(
    cursor: usize,
) -> Result<usize, OwnedRootBundleError> {
    cursor.checked_div(2).ok_or_else(arena::invariant)
}

/// Determine if an object edge cursor refers to a key (even) or value (odd).
pub(in crate::root_bundle) fn is_object_key_edge(
    cursor: usize,
) -> Result<bool, OwnedRootBundleError> {
    let rem = cursor.checked_rem(2).ok_or_else(arena::invariant)?;
    Ok(rem == 0)
}

/// Get the object fields array at the given index.
pub(in crate::root_bundle) fn object_fields_at(
    cp: &ValueStoreCheckpointV1,
    obj_idx: usize,
) -> Result<&[ObjectField], OwnedRootBundleError> {
    if obj_idx >= cp.objects.len() {
        return Err(arena::invariant());
    }
    cp.objects
        .get(obj_idx)
        .map(|fields| fields.as_ref())
        .ok_or_else(arena::invariant)
}

/// Get a specific field from an object's fields array.
/// Returns None if the field index is past the end.
pub(in crate::root_bundle) fn object_field_at(
    fields: &[ObjectField],
    field_idx: usize,
) -> Option<&ObjectField> {
    fields.get(field_idx)
}

/// Create an edge for an object field key.
pub(in crate::root_bundle) fn object_key_edge(field: &ObjectField) -> Edge {
    Edge {
        value: SlotValue::Symbol(field.key),
        taint: Taint::Clean,
    }
}

/// Create an edge for an object field value.
pub(in crate::root_bundle) fn object_value_edge(field: &ObjectField) -> Edge {
    Edge {
        value: field.value,
        taint: field.taint,
    }
}

/// Get the edge at the given cursor position for an object node.
/// Returns None if the cursor is past the end of the object's fields.
pub(in crate::root_bundle) fn object_edge(
    cp: &ValueStoreCheckpointV1,
    id: ObjectId,
    cursor: usize,
) -> Result<Option<Edge>, OwnedRootBundleError> {
    let id_val = id.get();
    let obj_idx = usize::try_from(id_val).map_err(|_| arena::invariant())?;
    let fields = object_fields_at(cp, obj_idx)?;
    let field_idx = object_field_index(cursor)?;
    let field = match object_field_at(fields, field_idx) {
        Some(f) => f,
        None => return Ok(None),
    };
    if is_object_key_edge(cursor)? {
        Ok(Some(object_key_edge(field)))
    } else {
        Ok(Some(object_value_edge(field)))
    }
}
