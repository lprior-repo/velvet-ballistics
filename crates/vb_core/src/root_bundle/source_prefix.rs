use crate::value_store::ValueStoreCheckpointV1;
use crate::workflow::source_metadata::WorkflowSourceMetadata;
use crate::{CoreError, CoreResult, SlotIdx, SlotValue, SymbolId, Taint};

/// Interns owned symbols by payload behind any immutable compiled prefix.
pub(super) fn relocate(
    roots: &mut [(SlotIdx, SlotValue, Taint)],
    checkpoint: &mut ValueStoreCheckpointV1,
    metadata: Option<&WorkflowSourceMetadata>,
) -> CoreResult<()> {
    let mut symbols = match metadata {
        Some(metadata) => metadata.symbols().to_vec(),
        None => Vec::new(),
    };
    let mut ids = Vec::new();
    ids.try_reserve_exact(checkpoint.symbols.len())
        .map_err(|_| CoreError::AllocationFailed)?;
    for text in &checkpoint.symbols {
        let index = if let Some(index) = symbols.iter().position(|value| value == text) {
            index
        } else {
            let index = symbols.len();
            symbols
                .try_reserve(1)
                .map_err(|_| CoreError::AllocationFailed)?;
            symbols.push(text.clone());
            index
        };
        ids.push(SymbolId::new(u32::try_from(index).map_err(|_| invalid())?));
    }
    for (_, value, _) in roots {
        remap(value, &ids)?;
    }
    for values in &mut checkpoint.lists {
        for value in values {
            remap(value, &ids)?;
        }
    }
    for fields in &mut checkpoint.objects {
        for field in fields {
            field.key = lookup(field.key, &ids)?;
            remap(&mut field.value, &ids)?;
        }
    }
    checkpoint.symbols = symbols;
    Ok(())
}
fn lookup(id: SymbolId, ids: &[SymbolId]) -> CoreResult<SymbolId> {
    ids.get(usize::try_from(id.get()).map_err(|_| invalid())?)
        .copied()
        .ok_or_else(invalid)
}
fn remap(value: &mut SlotValue, ids: &[SymbolId]) -> CoreResult<()> {
    if let SlotValue::Symbol(id) = value {
        *id = lookup(*id, ids)?;
    }
    Ok(())
}
fn invalid() -> CoreError {
    CoreError::InternalInvariantViolation {
        reason: "owned_source_symbol_relocation",
    }
}
