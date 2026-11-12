//! Applies field defaults to owned scratch payloads without renumbering handles.

use super::{MAX_INPUT_SCHEMA_DEPTH, materialize::intern, materialize_input_default};
use crate::limits::MAX_VALUES_PER_RUN;
use crate::value_store::{ValueStoreCheckpointV1, validate_object_len};
use crate::{
    CompiledInputConstraint, CompiledInputSlot, CoreError, CoreResult, InputSlotKind, ObjectField,
    ObjectId, SlotIdx, SlotValue, Taint, join_taint,
};

enum Target {
    Root,
    Field(ObjectId, usize),
}

enum Work<'a> {
    Enter(
        InputSlotKind,
        &'a CompiledInputConstraint,
        SlotValue,
        Taint,
        usize,
        Target,
    ),
    Leave(SlotValue, Taint, Target),
}

/// Fills nested defaults and returns the resulting root taint.
///
/// The checkpoint is caller-owned scratch and must be discarded on failure.
pub fn apply_input_schema_defaults(
    kind: InputSlotKind,
    constraint: &CompiledInputConstraint,
    value: SlotValue,
    taint: Taint,
    checkpoint: &mut ValueStoreCheckpointV1,
) -> CoreResult<Taint> {
    let mut root_taint = taint;
    converge(checkpoint, |checkpoint, remaining| {
        let (next, changed) =
            apply_pass(kind, constraint, value, root_taint, checkpoint, remaining)?;
        let changed = changed || next != root_taint;
        root_taint = next;
        Ok(changed)
    })?;
    Ok(root_taint)
}

pub(crate) fn apply_root_schema_defaults(
    roots: &mut [(SlotIdx, SlotValue, Taint)],
    declarations: &[CompiledInputSlot],
    checkpoint: &mut ValueStoreCheckpointV1,
) -> CoreResult<()> {
    converge(checkpoint, |checkpoint, remaining| {
        let mut changed = false;
        for declaration in declarations {
            let Some((_, value, taint)) = roots
                .iter_mut()
                .find(|(slot, _, _)| *slot == declaration.slot)
            else {
                continue;
            };
            let (next, fields_changed) = apply_pass(
                declaration.kind,
                &declaration.constraint,
                *value,
                *taint,
                checkpoint,
                remaining,
            )?;
            changed |= fields_changed || next != *taint;
            *taint = next;
        }
        Ok(changed)
    })
}

fn converge(
    checkpoint: &mut ValueStoreCheckpointV1,
    mut pass: impl FnMut(&mut ValueStoreCheckpointV1, &mut usize) -> CoreResult<bool>,
) -> CoreResult<()> {
    // Shared aliases may propagate taint into an earlier root. Only monotone
    // joins are allowed, and all passes share the schema traversal work cap.
    let mut remaining = MAX_VALUES_PER_RUN;
    while remaining > 0 {
        if !pass(checkpoint, &mut remaining)? {
            return Ok(());
        }
    }
    Err(limit())
}

fn apply_pass(
    kind: InputSlotKind,
    constraint: &CompiledInputConstraint,
    value: SlotValue,
    taint: Taint,
    checkpoint: &mut ValueStoreCheckpointV1,
    remaining: &mut usize,
) -> CoreResult<(Taint, bool)> {
    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| CoreError::AllocationFailed)?;
    work.push(Work::Enter(kind, constraint, value, taint, 0, Target::Root));
    let mut root_taint = taint;
    let mut changed = false;
    while let Some(task) = work.pop() {
        match task {
            Work::Leave(value, taint, target) => {
                let taint = payload_taint(value, taint, checkpoint)?;
                match target {
                    Target::Root => root_taint = taint,
                    Target::Field(id, index) => {
                        let fields = object_mut(checkpoint, id)?;
                        let field = fields.get_mut(index).ok_or_else(limit)?;
                        let next = join_taint(field.taint, taint);
                        changed |= next != field.taint;
                        field.taint = next;
                    }
                }
            }
            Work::Enter(kind, schema, value, taint, depth, target) => {
                *remaining = remaining.checked_sub(1).ok_or_else(limit)?;
                if depth > MAX_INPUT_SCHEMA_DEPTH {
                    return Err(limit());
                }
                let taint = if schema.secretness.is_secret() {
                    join_taint(taint, Taint::Secret)
                } else {
                    taint
                };
                work.try_reserve(1)
                    .map_err(|_| CoreError::AllocationFailed)?;
                work.push(Work::Leave(value, taint, target));
                let SlotValue::Object(id) = value else {
                    continue;
                };
                if !matches!(kind, InputSlotKind::Object | InputSlotKind::Any) {
                    continue;
                }
                let Some(declared) = schema.fields.as_deref() else {
                    continue;
                };
                for field in declared {
                    let index = find_field(checkpoint, id, &field.name)?;
                    let index = match index {
                        Some(index) => index,
                        None => {
                            let field_taint = if field.constraint.secretness.is_secret() {
                                join_taint(taint, Taint::Secret)
                            } else {
                                taint
                            };
                            let Some(value) =
                                default_value(&field.constraint, checkpoint, field_taint)?
                            else {
                                continue;
                            };
                            let key = intern(checkpoint, &field.name)?;
                            let existing = object(checkpoint, id)?;
                            let count = existing.len().checked_add(1).ok_or_else(limit)?;
                            validate_object_len(count)?;
                            let mut fields = Vec::new();
                            fields
                                .try_reserve_exact(count)
                                .map_err(|_| CoreError::AllocationFailed)?;
                            fields.extend_from_slice(existing);
                            let index = fields.len();
                            fields.push(ObjectField::with_taint(key, value, field_taint));
                            *object_mut(checkpoint, id)? = fields.into_boxed_slice();
                            changed = true;
                            index
                        }
                    };
                    let current = *object(checkpoint, id)?.get(index).ok_or_else(limit)?;
                    if work.len() >= MAX_VALUES_PER_RUN {
                        return Err(limit());
                    }
                    work.try_reserve(1)
                        .map_err(|_| CoreError::AllocationFailed)?;
                    work.push(Work::Enter(
                        field.kind,
                        &field.constraint,
                        current.value,
                        join_taint(current.taint, taint),
                        depth.checked_add(1).ok_or_else(limit)?,
                        Target::Field(id, index),
                    ));
                }
            }
        }
    }
    Ok((root_taint, changed))
}

/// Selects scalar defaults or materializes portable payload defaults.
pub fn default_value(
    schema: &CompiledInputConstraint,
    checkpoint: &mut ValueStoreCheckpointV1,
    taint: Taint,
) -> CoreResult<Option<SlotValue>> {
    match (schema.default_value, &schema.default_cold) {
        (Some(_), Some(_)) => Err(limit()),
        (
            Some(
                value @ (SlotValue::Null
                | SlotValue::Bool(_)
                | SlotValue::I64(_)
                | SlotValue::F64(_)),
            ),
            None,
        ) => Ok(Some(value)),
        (Some(_), None) => Err(limit()),
        (None, Some(value)) => materialize_input_default(value, checkpoint, taint).map(Some),
        (None, None) => Ok(None),
    }
}

fn find_field(cp: &ValueStoreCheckpointV1, id: ObjectId, name: &str) -> CoreResult<Option<usize>> {
    for (index, field) in object(cp, id)?.iter().enumerate() {
        let key = usize::try_from(field.key.get()).map_err(|_| limit())?;
        if cp.symbols.get(key).ok_or_else(limit)?.as_ref() == name {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

fn object(cp: &ValueStoreCheckpointV1, id: ObjectId) -> CoreResult<&[ObjectField]> {
    let index = usize::try_from(id.get()).map_err(|_| limit())?;
    cp.objects.get(index).map(AsRef::as_ref).ok_or_else(limit)
}

fn object_mut(
    cp: &mut ValueStoreCheckpointV1,
    id: ObjectId,
) -> CoreResult<&mut Box<[ObjectField]>> {
    let index = usize::try_from(id.get()).map_err(|_| limit())?;
    cp.objects.get_mut(index).ok_or_else(limit)
}

fn payload_taint(value: SlotValue, taint: Taint, cp: &ValueStoreCheckpointV1) -> CoreResult<Taint> {
    match value {
        SlotValue::Object(id) => Ok(object(cp, id)?
            .iter()
            .fold(taint, |joined, field| join_taint(joined, field.taint))),
        SlotValue::List(id) => {
            let index = usize::try_from(id.get()).map_err(|_| limit())?;
            Ok(cp
                .list_taints
                .get(index)
                .ok_or_else(limit)?
                .iter()
                .copied()
                .fold(taint, join_taint))
        }
        _ => Ok(taint),
    }
}

fn limit() -> CoreError {
    CoreError::ResourceLimitExceeded {
        resource: "input schema defaults",
    }
}
