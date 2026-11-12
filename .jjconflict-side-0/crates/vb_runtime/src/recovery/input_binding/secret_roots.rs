use super::*;

fn root_pairs(roots: &[(SlotIdx, SlotValue, Taint)]) -> RuntimeResult<Vec<(SlotIdx, SlotValue)>> {
    let mut pairs = Vec::new();
    pairs
        .try_reserve_exact(roots.len())
        .map_err(|_| vb_core::CoreError::AllocationFailed)?;
    pairs.extend(roots.iter().map(|(slot, value, _)| (*slot, *value)));
    Ok(pairs)
}

fn append_missing(
    self_: &mut RecoveredRetainedInputs,
    filled: &[(SlotIdx, SlotValue)],
) -> RuntimeResult<()> {
    for (slot, value) in filled {
        let present = self_.initial_inputs.iter().any(|(s, _, _)| *s == *slot);
        if !present {
            self_.append_secret_pair(*slot, *value)?;
        }
    }
    Ok(())
}

impl RecoveredRetainedInputs {
    fn append_secret_pair(&mut self, slot: SlotIdx, value: SlotValue) -> RuntimeResult<()> {
        self.initial_inputs
            .try_reserve(1)
            .map_err(|_| vb_core::CoreError::AllocationFailed)?;
        self.initial_inputs.push((slot, value, Taint::Secret));
        Ok(())
    }

    pub(crate) fn with_secret_inputs(
        mut self,
        shard: &crate::shard::Shard,
        workflow: &CompiledWorkflow,
    ) -> RuntimeResult<Self> {
        vb_core::workflow::source_metadata::secrets::validate_roots(
            workflow,
            &self.initial_inputs,
        )?;

        let pairs = root_pairs(&self.initial_inputs)?;
        let filled = shard.prepare_secret_inputs(workflow, &pairs)?;
        append_missing(&mut self, &filled)?;

        vb_core::workflow::source_metadata::secrets::validate_runtime_roots(
            workflow,
            &self.initial_inputs,
        )?;

        Ok(self)
    }
}
