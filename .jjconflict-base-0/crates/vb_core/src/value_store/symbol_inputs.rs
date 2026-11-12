#![forbid(unsafe_code)]
//! Checked cold-boundary composition of compiler and owned-input symbols.

use crate::errors::{CoreError, CoreResult};
use crate::ids::SymbolId;
use crate::limits::{MAX_SYMBOL_BYTES_PER_VALUE, MAX_VALUES_PER_RUN};
use crate::value::SlotValue;
use crate::workflow::source_metadata::{SourceMetadataError, WorkflowSourceMetadata};
use indexmap::IndexMap;
use thiserror::Error;

use super::{ObjectField, ValueStore, ValueStoreCheckpointV1};

/// Typed failure while preparing an input store with compiler symbols.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InputSymbolBindingError {
    /// Compiler or combined symbol metadata exceeds its contract.
    #[error("source metadata validation failed")]
    Metadata(#[source] SourceMetadataError),
    /// Input validation, scratch allocation, or store restoration failed.
    #[error("value store operation failed")]
    Core(#[source] CoreError),
}

type BindingResult<T> = Result<T, InputSymbolBindingError>;

#[derive(Default)]
struct SymbolTable<'a> {
    payloads: Vec<Box<str>>,
    by_bytes: IndexMap<&'a str, SymbolId>,
}

impl<'a> SymbolTable<'a> {
    fn intern(&mut self, text: &'a str) -> BindingResult<SymbolId> {
        if let Some(id) = self.by_bytes.get(text) {
            return Ok(*id);
        }
        let count = self
            .payloads
            .len()
            .checked_add(1)
            .ok_or(InputSymbolBindingError::Metadata(
                SourceMetadataError::SymbolIndexOverflow,
            ))?;
        check_symbol_count(count)?;
        let id = self
            .reserve_symbol()
            .map_err(InputSymbolBindingError::Core)?;
        let payload = copy_text(text).map_err(InputSymbolBindingError::Core)?;
        self.payloads.push(payload);
        self.by_bytes.insert(text, id);
        Ok(id)
    }

    fn reserve_symbol(&mut self) -> CoreResult<SymbolId> {
        let id = super::symbols::next_symbol_id(self.payloads.len())?;
        self.payloads
            .try_reserve(1)
            .map_err(|_| CoreError::AllocationFailed)?;
        self.by_bytes
            .try_reserve(1)
            .map_err(|_| CoreError::AllocationFailed)?;
        Ok(id)
    }
}

fn try_vec<T>(capacity: usize) -> CoreResult<Vec<T>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|_| CoreError::AllocationFailed)?;
    Ok(values)
}

fn copy_text(text: &str) -> CoreResult<Box<str>> {
    let mut copied = String::new();
    copied
        .try_reserve_exact(text.len())
        .map_err(|_| CoreError::AllocationFailed)?;
    copied.push_str(text);
    Ok(copied.into_boxed_str())
}

fn check_symbol_count(count: usize) -> BindingResult<()> {
    if count > MAX_VALUES_PER_RUN {
        return Err(InputSymbolBindingError::Metadata(
            SourceMetadataError::TooManySymbols {
                count,
                max: MAX_VALUES_PER_RUN,
            },
        ));
    }
    Ok(())
}

fn copy_compiler_symbol(index: usize, text: &str) -> BindingResult<Box<str>> {
    if text.len() <= MAX_SYMBOL_BYTES_PER_VALUE {
        return copy_text(text).map_err(InputSymbolBindingError::Core);
    }
    let id = u32::try_from(index)
        .map(SymbolId::new)
        .map_err(|_| InputSymbolBindingError::Metadata(SourceMetadataError::SymbolIndexOverflow))?;
    Err(InputSymbolBindingError::Metadata(
        SourceMetadataError::SymbolTooLong {
            id,
            bytes: text.len(),
            max: MAX_SYMBOL_BYTES_PER_VALUE,
        },
    ))
}

fn validate_prefix(prefix: &[Box<str>]) -> BindingResult<()> {
    check_symbol_count(prefix.len())?;
    let mut copied = try_vec(prefix.len()).map_err(InputSymbolBindingError::Core)?;
    prefix.iter().enumerate().try_for_each(|(index, text)| {
        copied.push(copy_compiler_symbol(index, text)?);
        Ok::<(), InputSymbolBindingError>(())
    })?;
    WorkflowSourceMetadata::try_new(copied.into_boxed_slice(), Box::default())
        .map(|_| ())
        .map_err(InputSymbolBindingError::Metadata)
}

fn validate_inputs(
    prefix: &[Box<str>],
    input: &ValueStoreCheckpointV1,
    roots: &[SlotValue],
) -> BindingResult<()> {
    validate_prefix(prefix)?;
    super::validate_checkpoint(input).map_err(InputSymbolBindingError::Core)?;
    roots
        .iter()
        .try_for_each(|value| super::validate_value_reference(*value, input))
        .map_err(InputSymbolBindingError::Core)
}

fn combined_symbols<'a>(
    prefix: &'a [Box<str>],
    input: &'a [Box<str>],
) -> BindingResult<(Vec<Box<str>>, Vec<SymbolId>)> {
    let mut table = SymbolTable::default();
    prefix
        .iter()
        .try_for_each(|text| table.intern(text).map(|_| ()))?;
    let mut old_to_new = try_vec(input.len()).map_err(InputSymbolBindingError::Core)?;
    input.iter().try_for_each(|text| {
        old_to_new.push(table.intern(text)?);
        Ok::<(), InputSymbolBindingError>(())
    })?;
    Ok((table.payloads, old_to_new))
}

fn remap_symbol(symbol: SymbolId, old_to_new: &[SymbolId]) -> CoreResult<SymbolId> {
    let index = super::symbols::symbol_index(symbol)?;
    old_to_new
        .get(index)
        .copied()
        .ok_or(CoreError::SymbolOutOfBounds { symbol })
}

fn remap_value(value: SlotValue, old_to_new: &[SymbolId]) -> CoreResult<SlotValue> {
    match value {
        SlotValue::Symbol(symbol) => remap_symbol(symbol, old_to_new).map(SlotValue::Symbol),
        other => Ok(other),
    }
}

fn remap_values(values: &mut [SlotValue], old_to_new: &[SymbolId]) -> CoreResult<()> {
    values.iter_mut().try_for_each(|value| {
        *value = remap_value(*value, old_to_new)?;
        Ok(())
    })
}

fn remap_fields(fields: &mut [ObjectField], old_to_new: &[SymbolId]) -> CoreResult<()> {
    fields.iter_mut().try_for_each(|field| {
        field.key = remap_symbol(field.key, old_to_new)?;
        field.value = remap_value(field.value, old_to_new)?;
        Ok(())
    })
}

fn copy_values<T: Copy>(values: &[T]) -> CoreResult<Box<[T]>> {
    let mut copied = try_vec(values.len())?;
    copied.extend_from_slice(values);
    Ok(copied.into_boxed_slice())
}

fn copy_arena<T: Copy>(arena: &[Box<[T]>]) -> CoreResult<Vec<Box<[T]>>> {
    let mut copied = try_vec(arena.len())?;
    arena.iter().try_for_each(|values| {
        copied.push(copy_values(values)?);
        Ok::<(), CoreError>(())
    })?;
    Ok(copied)
}

fn copy_blobs(blobs: &[bytes::Bytes]) -> CoreResult<Vec<bytes::Bytes>> {
    let mut copied = try_vec(blobs.len())?;
    copied.extend(blobs.iter().cloned());
    Ok(copied)
}

fn copy_checkpoint(
    input: &ValueStoreCheckpointV1,
    symbols: Vec<Box<str>>,
) -> CoreResult<ValueStoreCheckpointV1> {
    Ok(ValueStoreCheckpointV1 {
        symbols,
        lists: copy_arena(&input.lists)?,
        list_taints: copy_arena(&input.list_taints)?,
        objects: copy_arena(&input.objects)?,
        blobs: copy_blobs(&input.blobs)?,
        max_arena_entries: input.max_arena_entries,
    })
}

fn remap_checkpoint(
    mut checkpoint: ValueStoreCheckpointV1,
    old_to_new: &[SymbolId],
) -> CoreResult<ValueStoreCheckpointV1> {
    checkpoint
        .lists
        .iter_mut()
        .try_for_each(|values| remap_values(values, old_to_new))?;
    checkpoint
        .objects
        .iter_mut()
        .try_for_each(|fields| remap_fields(fields, old_to_new))?;
    Ok(checkpoint)
}

fn remap_roots(roots: &[SlotValue], old_to_new: &[SymbolId]) -> CoreResult<Box<[SlotValue]>> {
    let mut copied = copy_values(roots)?;
    remap_values(&mut copied, old_to_new)?;
    Ok(copied)
}

/// Prepares a new store and roots without changing the borrowed input state.
pub fn compose_compiler_symbols(
    compiler_symbols: &[Box<str>],
    input: &ValueStoreCheckpointV1,
    roots: &[SlotValue],
) -> Result<(ValueStore, Box<[SlotValue]>), InputSymbolBindingError> {
    validate_inputs(compiler_symbols, input, roots)?;
    let (symbols, old_to_new) = combined_symbols(compiler_symbols, &input.symbols)?;
    let checkpoint = copy_checkpoint(input, symbols).map_err(InputSymbolBindingError::Core)?;
    let checkpoint =
        remap_checkpoint(checkpoint, &old_to_new).map_err(InputSymbolBindingError::Core)?;
    let remapped_roots = remap_roots(roots, &old_to_new).map_err(InputSymbolBindingError::Core)?;
    let store =
        ValueStore::from_checkpoint_v1(checkpoint).map_err(InputSymbolBindingError::Core)?;
    Ok((store, remapped_roots))
}

#[cfg(test)]
#[path = "symbol_inputs_tests.rs"]
mod tests;
