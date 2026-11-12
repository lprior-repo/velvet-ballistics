use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::value::{SlotValue, Taint};
use bytes::Bytes;
use indexmap::IndexMap;

use super::ValueStore;
use super::blobs::{next_blob_id, validate_blob_len};
use super::lists::{next_list_id, validate_list_len};
use super::objects::{ObjectField, next_object_id, validate_object_len};
use super::symbols::{next_symbol_id, validate_symbol_len};

impl ValueStore {
    /// Inserts an interned symbol and returns its deterministic insertion ID.
    pub fn insert_symbol(&mut self, value: impl Into<Box<str>>) -> CoreResult<SymbolId> {
        let value = value.into();
        validate_symbol_len(value.len())?;
        self.check_arena_cap()?;
        let id = next_symbol_id(self.symbols.len())?;
        self.symbols.push(value);
        Ok(id)
    }

    /// Inserts a list payload and returns its deterministic insertion ID.
    /// All items are stored with `Taint::Clean`.
    pub fn insert_list(&mut self, values: impl Into<Box<[SlotValue]>>) -> CoreResult<ListId> {
        let values = values.into();
        validate_list_len(values.len())?;
        self.check_arena_cap()?;
        let id = next_list_id(self.lists.len())?;
        let taints = vec![Taint::Clean; values.len()].into_boxed_slice();
        self.list_taints.push(taints);
        self.lists.push(values);
        Ok(id)
    }

    /// Inserts a list payload with per-item taint and returns its deterministic insertion ID.
    pub fn insert_list_with_taint(
        &mut self,
        values: Box<[SlotValue]>,
        taints: Box<[Taint]>,
    ) -> CoreResult<ListId> {
        validate_list_len(values.len())?;
        if taints.len() != values.len() {
            return Err(CoreError::InternalInvariantViolation {
                reason: "list values and taints length mismatch".into(),
            });
        }
        self.check_arena_cap()?;
        let id = next_list_id(self.lists.len())?;
        self.list_taints.push(taints);
        self.lists.push(values);
        Ok(id)
    }

    /// Inserts object fields in caller-provided deterministic order.
    pub fn insert_object(&mut self, fields: impl Into<Box<[ObjectField]>>) -> CoreResult<ObjectId> {
        let fields = fields.into();
        validate_object_len(fields.len())?;
        self.check_arena_cap()?;
        let id = next_object_id(self.objects.len())?;
        let mut index = IndexMap::new();
        let mut taint_index = IndexMap::new();
        let mut field_pos = 0usize;
        while field_pos < fields.len() {
            let field = fields
                .get(field_pos)
                .ok_or(CoreError::InternalInvariantViolation {
                    reason: "object field index checked by loop bound".into(),
                })?;
            index.entry(field.key).or_insert(field.value);
            taint_index.entry(field.key).or_insert(field.taint);
            field_pos = field_pos
                .checked_add(1)
                .ok_or(CoreError::InternalInvariantViolation {
                    reason: "object field index overflow".into(),
                })?;
        }
        self.object_field_index.push(index);
        self.object_taint_index.push(taint_index);
        self.objects.push(fields);
        Ok(id)
    }

    /// Inserts a byte blob and returns its deterministic insertion ID.
    pub fn insert_blob(&mut self, bytes: impl Into<Bytes>) -> CoreResult<BlobId> {
        let bytes = bytes.into();
        validate_blob_len(bytes.len())?;
        self.check_arena_cap()?;
        let id = next_blob_id(self.blobs.len())?;
        self.blobs.push(bytes);
        Ok(id)
    }
}
