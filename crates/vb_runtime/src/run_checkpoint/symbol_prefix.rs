use super::{CompiledWorkflow, RunCheckpointV1Error, ValueStoreCheckpointV1};

pub(crate) fn validate_bound_symbol_prefix(
    workflow: &CompiledWorkflow,
    checkpoint: &ValueStoreCheckpointV1,
) -> Result<(), RunCheckpointV1Error> {
    let Some(prefix) = workflow.symbol_payloads() else {
        return Ok(());
    };
    prefix
        .iter()
        .enumerate()
        .try_for_each(|(offset, expected)| {
            let index =
                u32::try_from(offset).map_err(|_| RunCheckpointV1Error::InvalidNestedState)?;
            if checkpoint.symbols.get(offset) == Some(expected) {
                Ok(())
            } else {
                Err(RunCheckpointV1Error::SymbolPrefixMismatch { index })
            }
        })
}

#[cfg(test)]
#[path = "symbol_prefix_tests.rs"]
mod tests;
