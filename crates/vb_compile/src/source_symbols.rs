#![forbid(unsafe_code)]
//! Bounded cold-path symbol interning, not yet connected to source lowering.
//!
//! IDs follow first definition, never hash iteration order. The consuming array
//! is the exact symbol input for the approved `WorkflowSourceMetadata::try_new`
//! API. Runtime prefix seeding and artifact wiring remain separate prerequisites.

use std::collections::HashMap;
use vb_core::SymbolId;
use vb_core::limits::{MAX_SYMBOL_BYTES_PER_VALUE, MAX_VALUES_PER_RUN};

/// Typed failures contain only sizes and indices, never source text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum SourceSymbolError {
    #[error("symbol byte length {actual} exceeds {limit}")]
    SymbolTooLong { actual: usize, limit: usize },
    #[error("symbol count {count} has reached the limit {limit}")]
    SymbolCountLimit { count: usize, limit: usize },
    #[error("symbol count does not fit SymbolId")]
    SymbolIndexOverflow,
    #[error("symbol byte measurement overflow")]
    SymbolBytesOverflow,
    #[error("symbol table allocation reservation failed")]
    AllocationFailed,
    #[error("symbol table IDs are not dense in first-definition order")]
    NonDenseSymbolIds,
}

/// Owns each unique source string once; duplicate references allocate no IDs.
#[derive(Debug, Default)]
pub(crate) struct SourceSymbolTable {
    entries: HashMap<String, SymbolId>,
    symbol_bytes: u64,
}

impl SourceSymbolTable {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Rejects invalid additions before changing IDs, contents, or byte totals.
    /// Re-interning an existing value also works when the table is full.
    pub(crate) fn intern(&mut self, text: &str) -> Result<SymbolId, SourceSymbolError> {
        if let Some(id) = self.entries.get(text) {
            return Ok(*id);
        }
        validate_symbol_length(text.len())?;
        let id = checked_next_symbol_id(self.entries.len())?;
        let total = checked_symbol_bytes(self.symbol_bytes, text.len())?;
        let owned = copy_symbol(text)?;
        self.entries
            .try_reserve(1)
            .map_err(|_| SourceSymbolError::AllocationFailed)?;
        self.entries.insert(owned, id);
        self.symbol_bytes = total;
        Ok(id)
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn symbol_bytes(&self) -> u64 {
        self.symbol_bytes
    }

    /// Consumes the lookup index and returns one payload per ID, in ID order.
    /// Sorting numeric IDs avoids a second owned copy of every source string.
    pub(crate) fn finish(self) -> Result<Box<[Box<str>]>, SourceSymbolError> {
        let mut ordered = Vec::new();
        ordered
            .try_reserve_exact(self.entries.len())
            .map_err(|_| SourceSymbolError::AllocationFailed)?;
        ordered.extend(self.entries);
        ordered.sort_unstable_by_key(|(_, id)| id.get());
        let mut symbols = Vec::new();
        symbols
            .try_reserve_exact(ordered.len())
            .map_err(|_| SourceSymbolError::AllocationFailed)?;
        for (expected, (text, id)) in ordered.into_iter().enumerate() {
            let expected =
                u32::try_from(expected).map_err(|_| SourceSymbolError::SymbolIndexOverflow)?;
            if id.get() != expected {
                return Err(SourceSymbolError::NonDenseSymbolIds);
            }
            symbols.push(text.into_boxed_str());
        }
        Ok(symbols.into_boxed_slice())
    }
}

fn validate_symbol_length(actual: usize) -> Result<(), SourceSymbolError> {
    if actual > MAX_SYMBOL_BYTES_PER_VALUE {
        return Err(SourceSymbolError::SymbolTooLong {
            actual,
            limit: MAX_SYMBOL_BYTES_PER_VALUE,
        });
    }
    Ok(())
}

fn checked_next_symbol_id(count: usize) -> Result<SymbolId, SourceSymbolError> {
    if count >= MAX_VALUES_PER_RUN {
        return Err(SourceSymbolError::SymbolCountLimit {
            count,
            limit: MAX_VALUES_PER_RUN,
        });
    }
    u32::try_from(count)
        .map(SymbolId::new)
        .map_err(|_| SourceSymbolError::SymbolIndexOverflow)
}

fn checked_symbol_bytes(current: u64, additional: usize) -> Result<u64, SourceSymbolError> {
    let additional =
        u64::try_from(additional).map_err(|_| SourceSymbolError::SymbolBytesOverflow)?;
    current
        .checked_add(additional)
        .ok_or(SourceSymbolError::SymbolBytesOverflow)
}

fn copy_symbol(text: &str) -> Result<String, SourceSymbolError> {
    let mut owned = String::new();
    owned
        .try_reserve_exact(text.len())
        .map_err(|_| SourceSymbolError::AllocationFailed)?;
    owned.push_str(text);
    Ok(owned)
}

#[cfg(test)]
#[path = "source_symbols_tests.rs"]
mod tests;
