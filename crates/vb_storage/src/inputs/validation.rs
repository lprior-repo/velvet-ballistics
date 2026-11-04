use crate::JournalError;
use vb_core::{CompiledInputSlot, CompiledWorkflow, InputSlotKind, SlotIdx, SlotValue};

/// Validates supplied inputs against retained schema constraints and fills
/// scalar defaults for omitted slots.
///
/// Supplied pairs keep their order; filled defaults are appended in canonical
/// slot order. The input digest is order-independent, so fill order does not
/// affect admission identity.
pub(super) fn prepare_inputs(
    workflow: &CompiledWorkflow,
    inputs: &[(SlotIdx, SlotValue)],
) -> Result<Vec<(SlotIdx, SlotValue)>, JournalError> {
    if inputs.len() > workflow.input_slots().len() {
        return Err(JournalError::InputSchemaMismatch);
    }
    let mut remaining = declared_slots(workflow)?;
    let mut filled = Vec::new();
    filled
        .try_reserve_exact(inputs.len())
        .map_err(JournalError::AdmissionAllocationFailed)?;
    for (slot, value) in inputs {
        let declared = remaining
            .get_mut(usize::from(slot.get()))
            .and_then(Option::take)
            .ok_or(JournalError::InputSchemaMismatch)?;
        check_value(workflow, &declared, *value)?;
        filled.push((*slot, *value));
    }
    for slot in workflow.input_slots() {
        if remaining
            .get(usize::from(slot.slot.get()))
            .is_some_and(Option::is_some)
        {
            if slot.constraint.requires_value() {
                return Err(JournalError::InputSchemaMismatch);
            }
            if let Some(default) = slot.constraint.default_value {
                filled.push((slot.slot, default));
            }
        }
    }
    Ok(filled)
}

pub(super) fn validate_inputs(
    workflow: &CompiledWorkflow,
    inputs: &[(SlotIdx, SlotValue)],
) -> Result<(), JournalError> {
    if inputs.len() > workflow.input_slots().len() {
        return Err(JournalError::InputSchemaMismatch);
    }
    let mut remaining = declared_slots(workflow)?;
    for (slot, value) in inputs {
        let declared = remaining
            .get_mut(usize::from(slot.get()))
            .and_then(Option::take)
            .ok_or(JournalError::InputSchemaMismatch)?;
        check_value(workflow, &declared, *value)?;
    }
    for slot in workflow.input_slots() {
        if remaining
            .get(usize::from(slot.slot.get()))
            .is_some_and(Option::is_some)
            && slot.constraint.requires_value()
        {
            return Err(JournalError::InputSchemaMismatch);
        }
    }
    Ok(())
}

fn check_value(
    workflow: &CompiledWorkflow,
    declared: &CompiledInputSlot,
    value: SlotValue,
) -> Result<(), JournalError> {
    // A nullable slot accepts an explicit null without regard to kind.
    if !matches!(value, SlotValue::Null) || !declared.constraint.nullable {
        let kind = retained_value_kind(workflow, value)?;
        if declared.kind != InputSlotKind::Any && declared.kind != kind {
            return Err(JournalError::InputSchemaMismatch);
        }
    }
    if let SlotValue::I64(number) = value
        && (declared.constraint.min.is_some_and(|min| number < min)
            || declared.constraint.max.is_some_and(|max| number > max))
    {
        return Err(JournalError::InputSchemaMismatch);
    }
    Ok(())
}

fn declared_slots(
    workflow: &CompiledWorkflow,
) -> Result<Vec<Option<CompiledInputSlot>>, JournalError> {
    let mut remaining = Vec::new();
    remaining
        .try_reserve_exact(usize::from(workflow.slot_count()))
        .map_err(JournalError::AdmissionAllocationFailed)?;
    remaining.resize(usize::from(workflow.slot_count()), None);
    for declaration in workflow.input_slots() {
        let slot = remaining
            .get_mut(usize::from(declaration.slot.get()))
            .ok_or(JournalError::InputSchemaMismatch)?;
        *slot = Some(*declaration);
    }
    Ok(remaining)
}

fn retained_value_kind(
    workflow: &CompiledWorkflow,
    value: SlotValue,
) -> Result<InputSlotKind, JournalError> {
    match value {
        SlotValue::Null => Ok(InputSlotKind::Null),
        SlotValue::Bool(_) => Ok(InputSlotKind::Bool),
        SlotValue::I64(_) => Ok(InputSlotKind::I64),
        SlotValue::F64(_) => Ok(InputSlotKind::F64),
        SlotValue::Symbol(symbol) if symbol.get() < workflow.symbols_count() => {
            Ok(InputSlotKind::Symbol)
        }
        // The positional CLI wire carries handles, not their arena payloads.
        _ => Err(JournalError::InputSchemaMismatch),
    }
}
