#![forbid(unsafe_code)]

use super::{CoreError, CoreResult, SlotIdx, SymbolId, ValueStore};

// Source metadata projection; validation constructors and defaults are not fabricated.

// Production `crates/vb_core/src/workflow/source_metadata.rs:25-29`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledSecretRequirement {
    alias: SymbolId,
    requirement: SymbolId,
    slot: Option<SlotIdx>,
}

// Production `crates/vb_core/src/workflow/source_metadata.rs:54-59`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSourceMetadata {
    symbols: Box<[Box<str>]>,
    secret_requirements: Box<[CompiledSecretRequirement]>,
    symbols_count: u32,
    symbol_bytes: u64,
}

impl WorkflowSourceMetadata {
    // Production `crates/vb_core/src/workflow/source_metadata.rs:100-102`
    #[must_use]
    pub fn symbols(&self) -> &[Box<str>] {
        &self.symbols
    }
}

// Production `crates/vb_core/src/workflow/source_metadata.rs:143-146`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretNameRole {
    Alias,
    Requirement,
}

// Production `crates/vb_core/src/workflow/source_metadata.rs:152-199`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceMetadataError {
    TooManySymbols {
        count: usize,
        max: usize,
    },
    SymbolIndexOverflow,
    SymbolTooLong {
        id: SymbolId,
        bytes: usize,
        max: usize,
    },
    SymbolBytesOverflow,
    ValidationAllocationFailed,
    DuplicateSymbol {
        first: SymbolId,
        duplicate: SymbolId,
    },
    TooManyRequirements {
        count: usize,
        symbols: usize,
    },
    RequirementNameOutOfBounds {
        role: SecretNameRole,
        name: SymbolId,
    },
    EmptyRequirementName {
        role: SecretNameRole,
        name: SymbolId,
    },
    RequirementsNotStrictlyOrdered {
        previous: SymbolId,
        name: SymbolId,
    },
    SymbolCountMismatch {
        declared: u32,
        actual: u32,
    },
}

// Prefix runtime helper (sourcecopy, not proof): seed store from compiled prefix.
// Production `crates/vb_core/src/workflow/source_metadata/prefix.rs:5-24`
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

// Production `crates/vb_core/src/workflow/source_metadata/prefix.rs:26-30`
fn invalid() -> CoreError {
    CoreError::InternalInvariantViolation {
        reason: "compiled_source_symbol_prefix".into(),
    }
}
