#![forbid(unsafe_code)]
//! Artifact byte validation: re-parse from parts and verify checksums.

use crate::constants::MAX_COMPILED_IR_BYTES;
use crate::error::JournalError;

/// Size-gate decision before mapping to the broad storage error enum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CompiledIrSizeDecision {
    /// The encoded compiled-IR envelope fits the configured storage bound.
    WithinLimit,
    /// The encoded compiled-IR envelope exceeds the configured storage bound.
    PayloadTooLarge {
        /// Observed encoded length, saturated when the source length cannot fit u32.
        len: u32,
        /// Maximum accepted compiled-IR envelope length.
        max: u32,
    },
}

/// Validates a compiled workflow artifact's serialized bytes.
///
/// Performs:
/// 1. Structure validation: re-parse the workflow from serialized parts.
/// 2. Checksum validation: serialized bytes must hash to the claimed digest.
pub(crate) fn validate_workflow_artifact_bytes(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<Vec<u8>, JournalError> {
    let parts = workflow.to_parts();
    vb_core::CompiledWorkflow::try_from_parts(parts.clone())
        .map_err(|_| JournalError::ArtifactMalformed)?;
    let ir_bytes = canonical_workflow_ir_bytes(&parts)?;
    let computed = blake3::hash(&ir_bytes);
    if computed.as_bytes() == &workflow.digest().as_bytes() {
        Ok(ir_bytes)
    } else {
        Err(JournalError::ArtifactChecksumMismatch)
    }
}

/// Produces canonical BLAKE3-ready IR bytes from workflow parts.
///
/// The digest field is zeroed before serialization so that the hash is
/// stable regardless of the digest placeholder value.
pub(crate) fn canonical_workflow_ir_bytes(
    parts: &vb_core::WorkflowParts,
) -> Result<Vec<u8>, JournalError> {
    let mut parts_for_hash = parts.clone();
    parts_for_hash.digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    postcard::to_allocvec(&parts_for_hash).map_err(|_| JournalError::ArtifactMalformed)
}

/// Rejects compiled-IR envelope values larger than the configured storage bound.
pub fn reject_oversized_compiled_ir_value(len: usize) -> Result<(), JournalError> {
    match classify_compiled_ir_value_len(len) {
        CompiledIrSizeDecision::WithinLimit => Ok(()),
        CompiledIrSizeDecision::PayloadTooLarge { len, max } => {
            Err(JournalError::PayloadTooLarge { len, max })
        }
    }
}

pub(crate) fn classify_compiled_ir_value_len(len: usize) -> CompiledIrSizeDecision {
    let payload_len = match u32::try_from(len) {
        Ok(value) => value,
        Err(_) => {
            return CompiledIrSizeDecision::PayloadTooLarge {
                len: u32::MAX,
                max: MAX_COMPILED_IR_BYTES,
            };
        }
    };
    if payload_len > MAX_COMPILED_IR_BYTES {
        CompiledIrSizeDecision::PayloadTooLarge {
            len: payload_len,
            max: MAX_COMPILED_IR_BYTES,
        }
    } else {
        CompiledIrSizeDecision::WithinLimit
    }
}
