use super::WorkflowSourceMetadata;
use crate::{CoreError, CoreResult, SymbolId, ValueStore};

/// Installs payloads into an empty arena, or verifies an existing prefix.
pub fn seed_source_symbols(
    metadata: &WorkflowSourceMetadata,
    store: &mut ValueStore,
) -> CoreResult<()> {
    if store.symbol_count() == 0 {
        for (index, text) in metadata.symbols().iter().enumerate() {
            let expected = u32::try_from(index).map_err(|_| invalid())?;
            if store.insert_symbol(text.clone())? != SymbolId::new(expected) {
                return Err(invalid());
            }
        }
    }
    for (index, text) in metadata.symbols().iter().enumerate() {
        let id = SymbolId::new(u32::try_from(index).map_err(|_| invalid())?);
        if store.symbol(id)? != text.as_ref() {
            return Err(invalid());
        }
    }
    Ok(())
}

fn invalid() -> CoreError {
    CoreError::InternalInvariantViolation {
        reason: "compiled_source_symbol_prefix",
    }
}
