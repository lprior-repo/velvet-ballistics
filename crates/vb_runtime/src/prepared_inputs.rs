//! Pure input-only symbol preparation shared by consuming runtime routes.

use vb_core::value_store::{
    InputSymbolBindingError, ValueStoreCheckpointV1, compose_compiler_symbols,
};
use vb_core::{CompiledWorkflow, CoreError, SlotIdx, SlotValue, Taint, ValueStore};

type InputRoot = (SlotIdx, SlotValue, Taint);

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PreparedRuntimeInputs {
    pub(crate) store: ValueStore,
    pub(crate) roots: Vec<InputRoot>,
}

pub(crate) fn prepare_bound_runtime_inputs(
    workflow: &CompiledWorkflow,
    input_checkpoint: &ValueStoreCheckpointV1,
    roots: &[InputRoot],
) -> Result<Option<PreparedRuntimeInputs>, InputSymbolBindingError> {
    let Some(prefix) = workflow.symbol_payloads() else {
        return Ok(None);
    };
    let mut values = reserved_vec(roots.len())?;
    values.extend(roots.iter().map(|(_, value, _)| *value));
    let (store, values) = compose_compiler_symbols(prefix, input_checkpoint, &values)?;
    let mut prepared_roots = reserved_vec(roots.len())?;
    // The core composer preserves root count; retain tuple identity in that order.
    prepared_roots.extend(
        roots
            .iter()
            .zip(values.iter())
            .map(|((slot, _, taint), value)| (*slot, *value, *taint)),
    );
    Ok(Some(PreparedRuntimeInputs {
        store,
        roots: prepared_roots,
    }))
}

fn reserved_vec<T>(length: usize) -> Result<Vec<T>, InputSymbolBindingError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| InputSymbolBindingError::Core(CoreError::AllocationFailed))?;
    Ok(values)
}

#[cfg(test)]
#[path = "prepared_inputs/tests.rs"]
mod tests;
