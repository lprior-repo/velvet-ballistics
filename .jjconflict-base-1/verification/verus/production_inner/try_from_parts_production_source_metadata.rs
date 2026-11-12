#![forbid(unsafe_code)]

use super::SymbolId;

// Declaration-only source metadata projection: no validation body or default fabricated here.

// Production `crates/vb_core/src/workflow/source_metadata.rs:11-19`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledSecretRequirement {
    alias: SymbolId,
    requirement: SymbolId,
}

// Production `crates/vb_core/src/workflow/source_metadata.rs:43-54`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSourceMetadata {
    symbols: Vec<Box<str>>,
    secret_requirements: Vec<CompiledSecretRequirement>,
    symbols_count: u32,
    symbol_bytes: u64,
}

// Production `crates/vb_core/src/workflow/source_metadata.rs:136-143`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretNameRole {
    Alias,
    Requirement,
}

// Production `crates/vb_core/src/workflow/source_metadata.rs:145-194`
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
