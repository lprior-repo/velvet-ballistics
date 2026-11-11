use crate::prepared_inputs::prepare_bound_runtime_inputs;
use crate::{RuntimeError, RuntimeResult};
use vb_core::value_store::InputSymbolBindingError;
use vb_core::{CompiledWorkflow, CoreError, SlotIdx, SlotValue, Taint, ValueStore};

pub(super) fn prepare(
    workflow: &CompiledWorkflow,
    inputs: &[(SlotIdx, SlotValue)],
) -> RuntimeResult<Option<ValueStore>> {
    if workflow.symbol_payloads().is_none() {
        return Ok(None);
    }
    reject_storeless_handles(inputs)?;
    let roots = scalar_roots(inputs)?;
    let checkpoint =
        ValueStore::with_max_slots(workflow.resource_contract().max_slots).checkpoint_v1();
    // Direct inputs are scalars: composition preserves their values, so existing
    // slot seeding retains its kind validation and recovery-error semantics.
    prepare_bound_runtime_inputs(workflow, &checkpoint, &roots)
        .map(|prepared| prepared.map(|prepared| prepared.store))
        .map_err(RuntimeError::from)
}

fn reject_storeless_handles(inputs: &[(SlotIdx, SlotValue)]) -> RuntimeResult<()> {
    if inputs.iter().any(|(_, value)| {
        matches!(
            value,
            SlotValue::Symbol(_) | SlotValue::List(_) | SlotValue::Object(_) | SlotValue::Blob(_)
        )
    }) {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "bound_input_requires_owned_store",
        });
    }
    Ok(())
}

fn scalar_roots(
    inputs: &[(SlotIdx, SlotValue)],
) -> RuntimeResult<Vec<(SlotIdx, SlotValue, Taint)>> {
    let mut roots = Vec::new();
    roots.try_reserve_exact(inputs.len()).map_err(|_| {
        RuntimeError::from(InputSymbolBindingError::Core(CoreError::AllocationFailed))
    })?;
    roots.extend(
        inputs
            .iter()
            .map(|(slot, value)| (*slot, *value, Taint::Clean)),
    );
    Ok(roots)
}
