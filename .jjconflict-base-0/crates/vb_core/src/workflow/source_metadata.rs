#![forbid(unsafe_code)]
//! Immutable, validated source payloads awaiting the versioned workflow hook.
//!
//! This module neither loads secret values nor defines an artifact wire format.
//! Validation is a cold boundary operation, not an engine-transition operation.

use crate::ids::SymbolId;
use crate::limits::{MAX_SYMBOL_BYTES_PER_VALUE, MAX_VALUES_PER_RUN};
use thiserror::Error;

/// A presence-only secret binding referring to its owning symbol table.
///
/// This reference is validated by [`WorkflowSourceMetadata::try_new`]. It does
/// not authorize access, carry a value, or identify an ordinary input slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledSecretRequirement {
    alias: SymbolId,
    requirement: SymbolId,
}

impl CompiledSecretRequirement {
    /// Creates a table-relative requirement for later metadata validation.
    #[must_use]
    pub const fn new(alias: SymbolId, requirement: SymbolId) -> Self {
        Self { alias, requirement }
    }

    /// Returns the source mapping key, such as `token` in `token: TOKEN`.
    #[must_use]
    pub const fn alias(self) -> SymbolId {
        self.alias
    }

    /// Returns the declared requirement name, such as `TOKEN` in `token: TOKEN`.
    ///
    /// Interpretation of this name by an authorized host is a separate contract.
    #[must_use]
    pub const fn requirement(self) -> SymbolId {
        self.requirement
    }
}

/// Immutable source payloads in deterministic compiler-assigned identifier order.
///
/// Equal symbol bytes occur once. Requirement aliases are strictly increasing.
/// The compiler assigns identifiers; construction never sorts or renumbers the
/// payload table. Private derived measurements are not a wire representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSourceMetadata {
    symbols: Box<[Box<str>]>,
    secret_requirements: Box<[CompiledSecretRequirement]>,
    symbols_count: u32,
    symbol_bytes: u64,
}

impl WorkflowSourceMetadata {
    /// Takes ownership after validating payload bounds, identity, and requirements.
    ///
    /// The caller must bound decoding before allocating these input buffers.
    /// Validation reserves a bounded scratch vector fallibly, then sorts only
    /// borrowed references. A run may impose a smaller arena capacity separately.
    pub fn try_new(
        symbols: Box<[Box<str>]>,
        secret_requirements: Box<[CompiledSecretRequirement]>,
    ) -> Result<Self, SourceMetadataError> {
        if secret_requirements.len() > symbols.len() {
            return Err(SourceMetadataError::TooManyRequirements {
                count: secret_requirements.len(),
                symbols: symbols.len(),
            });
        }
        let (symbols_count, symbol_bytes) = validate_symbols(&symbols)?;
        validate_requirements(&symbols, &secret_requirements)?;
        Ok(Self {
            symbols,
            secret_requirements,
            symbols_count,
            symbol_bytes,
        })
    }

    /// Creates metadata for a workflow with no symbol payloads or requirements.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            symbols: Box::default(),
            secret_requirements: Box::default(),
            symbols_count: 0,
            symbol_bytes: 0,
        }
    }

    /// Returns immutable payloads in identifier order, without copying them.
    #[must_use]
    pub fn symbols(&self) -> &[Box<str>] {
        &self.symbols
    }

    /// Resolves an identifier only within this metadata's symbol table.
    #[must_use]
    pub fn symbol(&self, id: SymbolId) -> Option<&str> {
        let index = usize::try_from(id.get()).ok()?;
        self.symbols.get(index).map(Box::as_ref)
    }

    /// Returns the checked number of payloads, not a caller-supplied declaration.
    #[must_use]
    pub const fn symbols_count(&self) -> u32 {
        self.symbols_count
    }

    /// Returns total UTF-8 payload bytes, without imposing a new aggregate limit.
    #[must_use]
    pub const fn symbol_bytes(&self) -> u64 {
        self.symbol_bytes
    }

    /// Returns presence-only declarations in strictly increasing alias order.
    #[must_use]
    pub fn secret_requirements(&self) -> &[CompiledSecretRequirement] {
        &self.secret_requirements
    }

    /// Checks a future WorkflowParts symbol-count declaration against real payloads.
    pub fn validate_symbols_count(&self, declared: u32) -> Result<(), SourceMetadataError> {
        if declared != self.symbols_count {
            return Err(SourceMetadataError::SymbolCountMismatch {
                declared,
                actual: self.symbols_count,
            });
        }
        Ok(())
    }
}

/// Which side of a declared secret binding failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretNameRole {
    /// Source-language mapping key.
    Alias,
    /// Declared host requirement name, not its secret value.
    Requirement,
}

/// Expected failures while validating cold source metadata.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SourceMetadataError {
    /// The table exceeds the existing per-run outer arena bound.
    #[error("source symbol count {count} exceeds the arena limit {max}")]
    TooManySymbols { count: usize, max: usize },
    /// An identifier or count cannot be represented on this target.
    #[error("source symbol index or count cannot be represented")]
    SymbolIndexOverflow,
    /// One payload exceeds the existing per-symbol byte bound.
    #[error("source symbol {id:?} has {bytes} bytes, exceeding {max}")]
    SymbolTooLong {
        id: SymbolId,
        bytes: usize,
        max: usize,
    },
    /// Aggregate byte accounting cannot be represented.
    #[error("source symbol byte accounting overflowed")]
    SymbolBytesOverflow,
    /// A bounded validation scratch allocation was refused.
    #[error("source metadata validation scratch allocation failed")]
    ValidationAllocationFailed,
    /// Two distinct identifiers have the same UTF-8 payload.
    #[error("source symbols {first:?} and {duplicate:?} have identical bytes")]
    DuplicateSymbol {
        first: SymbolId,
        duplicate: SymbolId,
    },
    /// A unique requirement set cannot be larger than its name table.
    #[error("source requirement count {count} exceeds symbol count {symbols}")]
    TooManyRequirements { count: usize, symbols: usize },
    /// A requirement refers outside its owning table.
    #[error("secret {role:?} name {name:?} is outside the source symbol table")]
    RequirementNameOutOfBounds {
        role: SecretNameRole,
        name: SymbolId,
    },
    /// Text may be empty, but a presence requirement must name something.
    #[error("secret {role:?} name {name:?} is empty")]
    EmptyRequirementName {
        role: SecretNameRole,
        name: SymbolId,
    },
    /// Duplicate or descending requirement aliases are not canonical.
    #[error("secret requirement aliases are not strictly increasing: {previous:?}, {name:?}")]
    RequirementsNotStrictlyOrdered { previous: SymbolId, name: SymbolId },
    /// A separately declared count does not describe the validated table.
    #[error("declared symbol count {declared} differs from payload count {actual}")]
    SymbolCountMismatch { declared: u32, actual: u32 },
}

fn validate_symbols(symbols: &[Box<str>]) -> Result<(u32, u64), SourceMetadataError> {
    if symbols.len() > MAX_VALUES_PER_RUN {
        return Err(SourceMetadataError::TooManySymbols {
            count: symbols.len(),
            max: MAX_VALUES_PER_RUN,
        });
    }
    let count =
        u32::try_from(symbols.len()).map_err(|_| SourceMetadataError::SymbolIndexOverflow)?;
    let mut ordered: Vec<(SymbolId, &str)> = Vec::new();
    ordered
        .try_reserve_exact(symbols.len())
        .map_err(|_| SourceMetadataError::ValidationAllocationFailed)?;
    let mut total_bytes = 0_u64;
    for (index, payload) in symbols.iter().enumerate() {
        let id = SymbolId::new(
            u32::try_from(index).map_err(|_| SourceMetadataError::SymbolIndexOverflow)?,
        );
        if payload.len() > MAX_SYMBOL_BYTES_PER_VALUE {
            return Err(SourceMetadataError::SymbolTooLong {
                id,
                bytes: payload.len(),
                max: MAX_SYMBOL_BYTES_PER_VALUE,
            });
        }
        let bytes =
            u64::try_from(payload.len()).map_err(|_| SourceMetadataError::SymbolBytesOverflow)?;
        total_bytes = total_bytes
            .checked_add(bytes)
            .ok_or(SourceMetadataError::SymbolBytesOverflow)?;
        ordered.push((id, payload.as_ref()));
    }
    validate_unique_symbols(&mut ordered)?;
    Ok((count, total_bytes))
}

fn validate_unique_symbols(ordered: &mut [(SymbolId, &str)]) -> Result<(), SourceMetadataError> {
    ordered.sort_unstable_by(|left, right| left.1.cmp(right.1).then_with(|| left.0.cmp(&right.0)));
    for pair in ordered.windows(2) {
        if let [left, right] = pair
            && left.1 == right.1
        {
            return Err(SourceMetadataError::DuplicateSymbol {
                first: left.0,
                duplicate: right.0,
            });
        }
    }
    Ok(())
}

fn validate_requirements(
    symbols: &[Box<str>],
    requirements: &[CompiledSecretRequirement],
) -> Result<(), SourceMetadataError> {
    let mut previous = None;
    for requirement in requirements {
        let name = requirement.alias();
        validate_requirement_name(symbols, name, SecretNameRole::Alias)?;
        validate_requirement_name(
            symbols,
            requirement.requirement(),
            SecretNameRole::Requirement,
        )?;
        if let Some(previous) = previous
            && previous >= name
        {
            return Err(SourceMetadataError::RequirementsNotStrictlyOrdered { previous, name });
        }
        previous = Some(name);
    }
    Ok(())
}

fn validate_requirement_name(
    symbols: &[Box<str>],
    name: SymbolId,
    role: SecretNameRole,
) -> Result<(), SourceMetadataError> {
    let index = usize::try_from(name.get())
        .map_err(|_| SourceMetadataError::RequirementNameOutOfBounds { role, name })?;
    let payload = symbols
        .get(index)
        .ok_or(SourceMetadataError::RequirementNameOutOfBounds { role, name })?;
    if payload.is_empty() {
        return Err(SourceMetadataError::EmptyRequirementName { role, name });
    }
    Ok(())
}

#[cfg(test)]
#[path = "source_metadata/tests.rs"]
mod tests;
