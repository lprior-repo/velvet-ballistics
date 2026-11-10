use super::{InputDefaultValue, MAX_INPUT_SCHEMA_DEPTH};
use crate::limits::MAX_VALUES_PER_RUN;
use crate::value_store::{
    ValueStoreCheckpointV1, validate_list_len, validate_object_len, validate_symbol_len,
};
use crate::{CoreError, CoreResult, ListId, ObjectField, ObjectId, SlotValue, SymbolId, Taint};

enum Work<'a> {
    Value(&'a InputDefaultValue, usize),
    List(usize),
    Object(&'a [(Box<str>, InputDefaultValue)]),
}

/// Appends a bounded default tree to an owned scratch checkpoint.
///
/// Existing IDs remain stable. Discard the scratch checkpoint on failure;
/// partially appended payloads are never an accepted input bundle.
pub fn materialize_input_default(
    value: &InputDefaultValue,
    checkpoint: &mut ValueStoreCheckpointV1,
    taint: Taint,
) -> CoreResult<SlotValue> {
    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| CoreError::AllocationFailed)?;
    work.push(Work::Value(value, 0));
    let mut values = Vec::new();
    let mut visited = 0usize;
    while let Some(task) = work.pop() {
        let result = match task {
            Work::Value(value, depth) => {
                visited = visited.checked_add(1).ok_or_else(limit)?;
                if depth > MAX_INPUT_SCHEMA_DEPTH || visited > MAX_VALUES_PER_RUN {
                    return Err(limit());
                }
                match value {
                    InputDefaultValue::Null => SlotValue::Null,
                    InputDefaultValue::Bool(value) => SlotValue::Bool(*value),
                    InputDefaultValue::I64(value) => SlotValue::I64(*value),
                    InputDefaultValue::Text(text) => SlotValue::Symbol(intern(checkpoint, text)?),
                    InputDefaultValue::List(items) => {
                        validate_list_len(items.len())?;
                        reserve_work(&mut work, items.len())?;
                        work.push(Work::List(items.len()));
                        for item in items.iter().rev() {
                            work.push(Work::Value(item, depth.checked_add(1).ok_or_else(limit)?));
                        }
                        continue;
                    }
                    InputDefaultValue::Object(fields) => {
                        validate_object_len(fields.len())?;
                        reserve_work(&mut work, fields.len())?;
                        work.push(Work::Object(fields));
                        for (_, value) in fields.iter().rev() {
                            work.push(Work::Value(value, depth.checked_add(1).ok_or_else(limit)?));
                        }
                        continue;
                    }
                }
            }
            Work::List(count) => {
                let items = take_values(&mut values, count)?;
                reserve_entry(checkpoint)?;
                let id = ListId::new(u32::try_from(checkpoint.lists.len()).map_err(|_| limit())?);
                let mut taints = Vec::new();
                taints
                    .try_reserve_exact(count)
                    .map_err(|_| CoreError::AllocationFailed)?;
                taints.resize(count, taint);
                checkpoint
                    .lists
                    .try_reserve(1)
                    .map_err(|_| CoreError::AllocationFailed)?;
                checkpoint
                    .list_taints
                    .try_reserve(1)
                    .map_err(|_| CoreError::AllocationFailed)?;
                checkpoint.lists.push(items.into_boxed_slice());
                checkpoint.list_taints.push(taints.into_boxed_slice());
                SlotValue::List(id)
            }
            Work::Object(fields) => {
                let items = take_values(&mut values, fields.len())?;
                let mut output = Vec::new();
                output
                    .try_reserve_exact(fields.len())
                    .map_err(|_| CoreError::AllocationFailed)?;
                for ((key, _), value) in fields.iter().zip(items) {
                    output.push(ObjectField::with_taint(
                        intern(checkpoint, key)?,
                        value,
                        taint,
                    ));
                }
                reserve_entry(checkpoint)?;
                let id =
                    ObjectId::new(u32::try_from(checkpoint.objects.len()).map_err(|_| limit())?);
                checkpoint
                    .objects
                    .try_reserve(1)
                    .map_err(|_| CoreError::AllocationFailed)?;
                checkpoint.objects.push(output.into_boxed_slice());
                SlotValue::Object(id)
            }
        };
        values
            .try_reserve(1)
            .map_err(|_| CoreError::AllocationFailed)?;
        values.push(result);
    }
    values.pop().ok_or_else(limit)
}

pub(crate) fn intern(checkpoint: &mut ValueStoreCheckpointV1, text: &str) -> CoreResult<SymbolId> {
    validate_symbol_len(text.len())?;
    if let Some(index) = checkpoint
        .symbols
        .iter()
        .position(|value| value.as_ref() == text)
    {
        return u32::try_from(index).map(SymbolId::new).map_err(|_| limit());
    }
    reserve_entry(checkpoint)?;
    let id = SymbolId::new(u32::try_from(checkpoint.symbols.len()).map_err(|_| limit())?);
    let mut owned = String::new();
    owned
        .try_reserve_exact(text.len())
        .map_err(|_| CoreError::AllocationFailed)?;
    owned.push_str(text);
    checkpoint
        .symbols
        .try_reserve(1)
        .map_err(|_| CoreError::AllocationFailed)?;
    checkpoint.symbols.push(owned.into_boxed_str());
    Ok(id)
}

fn reserve_entry(checkpoint: &ValueStoreCheckpointV1) -> CoreResult<()> {
    let mut count = 0u64;
    for length in [
        checkpoint.symbols.len(),
        checkpoint.lists.len(),
        checkpoint.objects.len(),
        checkpoint.blobs.len(),
    ] {
        count = count
            .checked_add(u64::try_from(length).map_err(|_| limit())?)
            .ok_or_else(limit)?;
    }
    if (checkpoint.max_arena_entries > 0 && count >= checkpoint.max_arena_entries)
        || count >= u64::try_from(MAX_VALUES_PER_RUN).map_err(|_| limit())?
    {
        return Err(limit());
    }
    Ok(())
}

fn reserve_work(work: &mut Vec<Work<'_>>, count: usize) -> CoreResult<()> {
    let extra = count.checked_add(1).ok_or_else(limit)?;
    if work.len().checked_add(extra).ok_or_else(limit)? > MAX_VALUES_PER_RUN {
        return Err(limit());
    }
    work.try_reserve(extra)
        .map_err(|_| CoreError::AllocationFailed)
}

fn take_values(values: &mut Vec<SlotValue>, count: usize) -> CoreResult<Vec<SlotValue>> {
    let start = values.len().checked_sub(count).ok_or_else(limit)?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(count)
        .map_err(|_| CoreError::AllocationFailed)?;
    result.extend(values.drain(start..));
    Ok(result)
}

fn limit() -> CoreError {
    CoreError::ResourceLimitExceeded {
        resource: "input defaults",
    }
}
