use crate::errors::{CoreError, CoreResult};
use crate::ids::SymbolId;
use crate::limits::MAX_SYMBOL_BYTES_PER_VALUE;

pub fn next_symbol_id(len: usize) -> CoreResult<SymbolId> {
    u32::try_from(len)
        .map(SymbolId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded {
            resource: "symbols",
        })
}

pub fn validate_symbol_len(len: usize) -> CoreResult<()> {
    if len > MAX_SYMBOL_BYTES_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "symbol_bytes",
        })
    } else {
        Ok(())
    }
}

pub fn symbol_index(id: SymbolId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::SymbolOutOfBounds { symbol: id })
}
