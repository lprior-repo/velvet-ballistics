use crate::JournalError;
use vb_core::{CompiledInputSlot, CompiledWorkflow, SlotIdx, SlotValue};

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
                check_value(workflow, slot, default)?;
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
    vb_core::input_validation::validate_input_value(declared, value, workflow.symbols_count(), None)
        .map_err(|_| JournalError::InputSchemaMismatch)
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
