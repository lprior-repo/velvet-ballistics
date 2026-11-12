#![forbid(unsafe_code)]
//! Bound symbol table representation and versioned workflow IR parts.
//!
//! This module provides the V2 workflow IR representation carrying the
//! ordered symbol payload table. It stores validated source metadata
//! privately and exposes an immutable projection.

use super::{CompiledWorkflow, WorkflowError, WorkflowParts, source_metadata};
use serde::{Deserialize, Serialize};

/// V2 workflow IR with explicit symbol payloads.
///
/// `wire_version` is 2. `symbol_payloads` contains one unique UTF-8
/// payload per symbol ID, in deterministic compiler-assigned order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowIrV2 {
    /// Wire format version, currently 2.
    pub wire_version: u16,
    /// Underlying workflow parts.
    pub parts: WorkflowParts,
    /// Ordered symbol payloads, one per compiler-assigned symbol ID.
    pub symbol_payloads: Box<[Box<str>]>,
}

/// Discriminator for versioned workflow parts.
///
/// `V1` carries the legacy parts-only representation. `V2` carries
/// the explicit bound symbol payload table. Neither is a wire format;
/// serialization is handled by the codec layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionedWorkflowParts {
    /// Legacy V1 representation without explicit symbol payloads.
    V1(WorkflowParts),
    /// V2 representation with bound symbol payloads.
    V2(WorkflowIrV2),
}

impl CompiledWorkflow {
    /// Reconstructs a bound V2 workflow from validated parts and symbol payloads.
    ///
    /// Validates the ordered payload table via [`source_metadata::WorkflowSourceMetadata`],
    /// then runs existing parts validation via [`Self::try_from_parts`].
    /// Returns a workflow carrying the validated symbol payloads as
    /// immutable private state. `None` means legacy V1; `Some(empty)` is
    /// a valid explicitly bound V2 with zero symbols.
    pub fn try_from_parts_with_symbols(
        parts: WorkflowParts,
        symbol_payloads: Box<[Box<str>]>,
    ) -> Result<Self, WorkflowError> {
        let metadata =
            source_metadata::WorkflowSourceMetadata::try_new(symbol_payloads, Box::default())
                .map_err(|source| WorkflowError::InvalidSymbolMetadata { source })?;
        metadata
            .validate_symbols_count(parts.symbols_count)
            .map_err(|source| WorkflowError::InvalidSymbolMetadata { source })?;
        let mut workflow = Self::try_from_parts(parts)?;
        workflow.source_symbols = Some(metadata);
        Ok(workflow)
    }

    /// Returns the versioned workflow parts representation for serialization.
    ///
    /// Legacy V1 workflows return `V1(parts)`. Workflows carrying
    /// symbol payloads return `V2(WorkflowIrV2)` with wire version 2.
    #[must_use]
    pub fn to_versioned_parts(&self) -> VersionedWorkflowParts {
        let parts = self.to_parts();
        if let Some(metadata) = self.source_symbols.as_ref() {
            VersionedWorkflowParts::V2(WorkflowIrV2 {
                wire_version: 2,
                parts,
                symbol_payloads: Box::from(metadata.symbols()),
            })
        } else {
            VersionedWorkflowParts::V1(parts)
        }
    }

    /// Returns bound symbol payloads, or `None` for legacy V1.
    ///
    /// `Some(&[])` denotes a V2 workflow with zero symbols.
    #[must_use]
    pub fn symbol_payloads(&self) -> Option<&[Box<str>]> {
        self.source_symbols
            .as_ref()
            .map(source_metadata::WorkflowSourceMetadata::symbols)
    }
}

#[cfg(test)]
#[path = "bound_symbols_tests.rs"]
mod tests;
