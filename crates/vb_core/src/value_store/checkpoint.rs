#![forbid(unsafe_code)]
//! Versioned checkpoint representation for the complete cold value store.

use serde::{Deserialize, Serialize};

use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::value::{SlotValue, Taint};

use super::{ObjectField, ValueStore};

/// Canonical v1 representation of every value arena and its configured bound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueStoreCheckpointV1 {
    /// Interned symbols in identifier order.
    pub symbols: Vec<Box<str>>,
    /// Lists in identifier order.
    pub lists: Vec<Box<[SlotValue]>>,
    /// Per-item list taint in the same order as `lists`.
    pub list_taints: Vec<Box<[Taint]>>,
    /// Objects in identifier order and field insertion order.
    pub objects: Vec<Box<[ObjectField]>>,
    /// Blob payloads in identifier order.
    pub blobs: Vec<bytes::Bytes>,
    /// Arena entry bound; zero means unlimited.
    pub max_arena_entries: u64,
}

impl ValueStore {
    /// Copies all canonical value arenas into the versioned checkpoint DTO.
    #[must_use]
    pub fn checkpoint_v1(&self) -> ValueStoreCheckpointV1 {
        ValueStoreCheckpointV1 {
            symbols: self.symbols.clone(),
            lists: self.lists.clone(),
            list_taints: self.list_taints.clone(),
            objects: self.objects.clone(),
            blobs: self.blobs.clone(),
            max_arena_entries: self.max_arena_entries,
        }
    }

    /// Rebuilds secondary indexes after validating arena bounds and references.
    pub fn from_checkpoint_v1(checkpoint: ValueStoreCheckpointV1) -> CoreResult<Self> {
        validate_checkpoint(&checkpoint)?;
        let mut object_field_index = Vec::with_capacity(checkpoint.objects.len());
        let mut object_taint_index = Vec::with_capacity(checkpoint.objects.len());
        for fields in &checkpoint.objects {
            let mut field_index = indexmap::IndexMap::new();
            let mut taint_index = indexmap::IndexMap::new();
            for field in fields.iter() {
                field_index.entry(field.key).or_insert(field.value);
                taint_index.entry(field.key).or_insert(field.taint);
            }
            object_field_index.push(field_index);
            object_taint_index.push(taint_index);
        }
        Ok(Self {
            symbols: checkpoint.symbols,
            lists: checkpoint.lists,
            list_taints: checkpoint.list_taints,
            objects: checkpoint.objects,
            object_field_index,
            object_taint_index,
            blobs: checkpoint.blobs,
            max_arena_entries: checkpoint.max_arena_entries,
        })
    }
}

fn validate_checkpoint(checkpoint: &ValueStoreCheckpointV1) -> CoreResult<()> {
    if checkpoint.lists.len() != checkpoint.list_taints.len() {
        return Err(invalid_checkpoint("list_taint_count_mismatch"));
    }
    for (list, taints) in checkpoint.lists.iter().zip(&checkpoint.list_taints) {
        super::validate_list_len(list.len())?;
        if list.len() != taints.len() {
            return Err(invalid_checkpoint("list_taint_length_mismatch"));
        }
        for value in list.iter() {
            validate_value_reference(*value, checkpoint)?;
        }
    }
    for symbol in &checkpoint.symbols {
        super::validate_symbol_len(symbol.len())?;
    }
    for object in &checkpoint.objects {
        super::validate_object_len(object.len())?;
        for field in object.iter() {
            validate_symbol_reference(field.key, checkpoint)?;
            validate_value_reference(field.value, checkpoint)?;
        }
    }
    for blob in &checkpoint.blobs {
        super::validate_blob_len(blob.len())?;
    }
    if checkpoint.max_arena_entries > 0 && arena_count(checkpoint)? > checkpoint.max_arena_entries {
        return Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: checkpoint.max_arena_entries,
        });
    }
    Ok(())
}

fn validate_value_reference(
    value: SlotValue,
    checkpoint: &ValueStoreCheckpointV1,
) -> CoreResult<()> {
    match value {
        SlotValue::Symbol(id) => validate_symbol_reference(id, checkpoint),
        SlotValue::List(id) => validate_list_reference(id, checkpoint),
        SlotValue::Object(id) => validate_object_reference(id, checkpoint),
        SlotValue::Blob(id) => validate_blob_reference(id, checkpoint),
        SlotValue::Null | SlotValue::Bool(_) | SlotValue::I64(_) | SlotValue::F64(_) => Ok(()),
    }
}

fn validate_symbol_reference(id: SymbolId, checkpoint: &ValueStoreCheckpointV1) -> CoreResult<()> {
    let index =
        usize::try_from(id.get()).map_err(|_| CoreError::SymbolOutOfBounds { symbol: id })?;
    checkpoint
        .symbols
        .get(index)
        .map(|_| ())
        .ok_or(CoreError::SymbolOutOfBounds { symbol: id })
}

fn validate_list_reference(id: ListId, checkpoint: &ValueStoreCheckpointV1) -> CoreResult<()> {
    let index = usize::try_from(id.get()).map_err(|_| CoreError::ListOutOfBounds { list: id })?;
    checkpoint
        .lists
        .get(index)
        .map(|_| ())
        .ok_or(CoreError::ListOutOfBounds { list: id })
}

fn validate_object_reference(id: ObjectId, checkpoint: &ValueStoreCheckpointV1) -> CoreResult<()> {
    let index =
        usize::try_from(id.get()).map_err(|_| CoreError::ObjectOutOfBounds { object: id })?;
    checkpoint
        .objects
        .get(index)
        .map(|_| ())
        .ok_or(CoreError::ObjectOutOfBounds { object: id })
}

fn validate_blob_reference(id: BlobId, checkpoint: &ValueStoreCheckpointV1) -> CoreResult<()> {
    let index = usize::try_from(id.get()).map_err(|_| CoreError::BlobOutOfBounds { blob: id })?;
    checkpoint
        .blobs
        .get(index)
        .map(|_| ())
        .ok_or(CoreError::BlobOutOfBounds { blob: id })
}

fn arena_count(checkpoint: &ValueStoreCheckpointV1) -> CoreResult<u64> {
    let mut total = 0_u64;
    for count in [
        checkpoint.symbols.len(),
        checkpoint.lists.len(),
        checkpoint.objects.len(),
        checkpoint.blobs.len(),
    ] {
        let count = u64::try_from(count).map_err(|_| invalid_checkpoint("arena_count_overflow"))?;
        total = total
            .checked_add(count)
            .ok_or_else(|| invalid_checkpoint("arena_count_overflow"))?;
    }
    Ok(total)
}

fn invalid_checkpoint(reason: &'static str) -> CoreError {
    CoreError::InternalInvariantViolation { reason }
}

#[cfg(test)]
mod tests {
    use super::{ValueStore, ValueStoreCheckpointV1};
    use crate::value::{SlotValue, Taint};
    use crate::value_store::ObjectField;

    #[test]
    fn value_store_checkpoint_roundtrips_all_arenas_and_taints() -> Result<(), String> {
        let mut store = ValueStore::with_max_slots(8);
        let symbol = store
            .insert_symbol("name")
            .map_err(|error| format!("symbol setup failed: {error}"))?;
        let list = store
            .insert_list_with_taint(
                Box::from([SlotValue::I64(4)]),
                Box::from([Taint::DerivedFromSecret]),
            )
            .map_err(|error| format!("list setup failed: {error}"))?;
        let object = store
            .insert_object(Box::from([ObjectField::with_taint(
                symbol,
                SlotValue::List(list),
                Taint::Secret,
            )]))
            .map_err(|error| format!("object setup failed: {error}"))?;
        store
            .insert_blob(vec![1_u8, 2_u8, 3_u8])
            .map_err(|error| format!("blob setup failed: {error}"))?;

        let checkpoint = store.checkpoint_v1();
        let encoded = postcard::to_allocvec(&checkpoint)
            .map_err(|error| format!("checkpoint encode failed: {error}"))?;
        let checkpoint: ValueStoreCheckpointV1 = postcard::from_bytes(&encoded)
            .map_err(|error| format!("checkpoint decode failed: {error}"))?;
        let restored = ValueStore::from_checkpoint_v1(checkpoint.clone())
            .map_err(|error| format!("checkpoint restore failed: {error}"))?;

        assert_eq!(restored.checkpoint_v1(), checkpoint);
        assert_eq!(
            restored.object_field(object, symbol),
            Ok(SlotValue::List(list))
        );
        assert_eq!(
            restored.object_field_with_taint(object, symbol),
            Ok((SlotValue::List(list), Taint::Secret))
        );
        Ok(())
    }

    #[test]
    fn value_store_checkpoint_rejects_broken_list_taint_shape() {
        let store = ValueStore::with_max_slots(4);
        let mut checkpoint = store.checkpoint_v1();
        checkpoint.lists.push(Box::from([SlotValue::I64(1)]));
        checkpoint.list_taints.push(Box::from([]));

        assert!(ValueStore::from_checkpoint_v1(checkpoint).is_err());
    }
}
