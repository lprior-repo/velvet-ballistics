use super::*;

fn invalid(error: &str) -> RuntimeError {
    RuntimeError::Recovery {
        error: error.into(),
    }
}

fn declaration_taints(workflow: &CompiledWorkflow) -> RuntimeResult<Vec<Option<Taint>>> {
    let slots = usize::from(workflow.slot_count());
    let mut taints = Vec::new();
    taints
        .try_reserve_exact(slots)
        .map_err(|_| invalid("retained declaration capacity"))?;
    taints.resize(slots, None);
    workflow.input_slots().iter().try_for_each(|input| {
        let target = taints
            .get_mut(input.slot.as_usize())
            .ok_or_else(|| invalid("retained declaration slot"))?;
        *target = Some(if input.constraint.secretness.is_secret() {
            Taint::Secret
        } else {
            Taint::Clean
        });
        Ok::<(), RuntimeError>(())
    })?;
    Ok(taints)
}

pub(super) fn scalar_roots(
    record: &RunInputsRecord,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<Vec<(SlotIdx, SlotValue, Taint)>> {
    let decls = declaration_taints(workflow)?;
    let inputs = record.inputs();
    let mut roots = Vec::new();
    roots
        .try_reserve_exact(inputs.len())
        .map_err(|_| invalid("retained input capacity"))?;
    inputs.iter().try_for_each(|(slot, value)| {
        let taint = decls
            .get(slot.as_usize())
            .copied()
            .flatten()
            .ok_or_else(|| invalid("retained input declaration missing"))?;
        roots.push((*slot, *value, taint));
        Ok::<(), RuntimeError>(())
    })?;
    Ok(roots)
}
