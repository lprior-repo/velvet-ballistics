#![forbid(unsafe_code)]
//! Compiled IR encoding, versioned payloads, and canonical digests.
//!
//! Provides version-aware encoding of `VersionedWorkflowParts` with a stable
//! prefix scheme and canonical preimage computation for digesting.

use crate::mod_compile_errors::{CompileError, CompileErrors};
use vb_core::workflow::VersionedWorkflowParts;
use vb_core::{CompiledWorkflow, WorkflowDigest};

/// V2 payload literal prefix.
const V2_PREFIX: [u8; 4] = [1, 255, 86, 66];

/// Encode a compiled workflow into versioned payload bytes.
///
/// V1 workflows emit raw postcard-encoded `WorkflowParts`. V2 workflows
/// emit the `V2_PREFIX` followed by postcard-encoded `WorkflowIrV2`.
/// Serialization failures return `CompileErrors` with `CompiledIrEncode`.
pub fn encode_compiled_ir(workflow: &CompiledWorkflow) -> Result<Box<[u8]>, CompileErrors> {
    let parts = workflow.to_versioned_parts();
    encode_versioned_parts(&parts)
        .map(Vec::into_boxed_slice)
        .map_err(|error| CompileErrors(vec![CompileError::CompiledIrEncode(error)]))
}

/// Compute a canonical digest for versioned workflow parts.
///
/// Computes the BLAKE3 hash of the canonical preimage with the nested
/// `parts.digest` zeroed. Works for both V1 and V2 representations.
pub fn compute_compiled_ir_digest(
    parts: &VersionedWorkflowParts,
) -> Result<WorkflowDigest, CompileErrors> {
    let preimage = canonical_preimage(parts)
        .map_err(|error| CompileErrors(vec![CompileError::CompiledIrEncode(error)]))?;
    let hash = blake3::hash(&preimage);
    Ok(WorkflowDigest::from_bytes(hash.into()))
}

/// Internal versioned payload encoder for both V1 and V2.
///
/// Never wraps the versioned enum directly; encodes the inner DTO.
pub(crate) fn encode_versioned_parts(
    parts: &VersionedWorkflowParts,
) -> Result<Vec<u8>, postcard::Error> {
    match parts {
        VersionedWorkflowParts::V1(wparts) => postcard::to_allocvec(wparts),
        VersionedWorkflowParts::V2(ir) => {
            let mut output = V2_PREFIX.to_vec();
            output.extend_from_slice(&postcard::to_allocvec(ir)?);
            Ok(output)
        }
    }
}

/// Compute the canonical preimage for digesting versioned parts.
///
/// Clones the versioned DTO, zeros the nested `parts.digest` field in both
/// variants, and encodes through the same versioned encoder used for payloads.
pub(crate) fn canonical_preimage(
    parts: &VersionedWorkflowParts,
) -> Result<Vec<u8>, postcard::Error> {
    let versioned = match parts {
        VersionedWorkflowParts::V1(wparts) => {
            let mut cloned = wparts.clone();
            cloned.digest = WorkflowDigest::from_bytes([0; 32]);
            VersionedWorkflowParts::V1(cloned)
        }
        VersionedWorkflowParts::V2(ir) => {
            let mut cloned_ir = ir.clone();
            cloned_ir.parts.digest = WorkflowDigest::from_bytes([0; 32]);
            VersionedWorkflowParts::V2(cloned_ir)
        }
    };
    encode_versioned_parts(&versioned)
}

#[cfg(test)]
#[path = "compiled_ir_tests.rs"]
mod tests;
