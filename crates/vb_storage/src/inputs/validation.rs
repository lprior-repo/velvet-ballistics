use crate::JournalError;
use vb_core::{CompiledWorkflow, InputSlotKind, SlotIdx, SlotValue};

pub(super) fn validate_inputs(
    workflow: &CompiledWorkflow,
    inputs: &[(SlotIdx, SlotValue)],
) -> Result<(), JournalError> {
    if inputs.len() > workflow.input_slots().len() {
        return Err(JournalError::InputSchemaMismatch);
    }
    let mut remaining = declared_kinds(workflow)?;
    for (slot, value) in inputs {
        let expected = remaining
            .get_mut(usize::from(slot.get()))
            .and_then(Option::take)
            .ok_or(JournalError::InputSchemaMismatch)?;
        let kind = retained_value_kind(workflow, *value)?;
        if expected != InputSlotKind::Any && expected != kind {
            return Err(JournalError::InputSchemaMismatch);
        }
    }
    Ok(())
}

fn declared_kinds(workflow: &CompiledWorkflow) -> Result<Vec<Option<InputSlotKind>>, JournalError> {
    let mut remaining = Vec::new();
    remaining
        .try_reserve_exact(usize::from(workflow.slot_count()))
        .map_err(JournalError::AdmissionAllocationFailed)?;
    remaining.resize(usize::from(workflow.slot_count()), None);
    for declaration in workflow.input_slots() {
        let slot = remaining
            .get_mut(usize::from(declaration.slot.get()))
            .ok_or(JournalError::InputSchemaMismatch)?;
        *slot = Some(declaration.kind);
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
