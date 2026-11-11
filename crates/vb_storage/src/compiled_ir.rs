#![forbid(unsafe_code)]
//! Canonical V1 and bound-symbol V2 compiled workflow encoding.

use serde::{Serialize, de::DeserializeOwned};
use vb_core::ids::WorkflowDigest;
use vb_core::workflow::{CompiledWorkflow, VersionedWorkflowParts, WorkflowIrV2, WorkflowParts};

use crate::compat::{
    CURRENT_IR_VERSION, CompatibilityClass, SchemaVersion, artifact_ir_version_inputs_for, classify,
};
use crate::constants::MAX_COMPILED_IR_BYTES;
use crate::error::JournalError;

const V2_PREFIX: [u8; 4] = [0x01, 0xff, b'V', b'B'];
const V2_VERSION: SchemaVersion = SchemaVersion::new(2, 0, 0);

/// Encodes the selected wire representation without replacing a stale digest.
pub fn encode_compiled_ir(workflow: &CompiledWorkflow) -> Result<Box<[u8]>, JournalError> {
    match workflow.to_versioned_parts() {
        VersionedWorkflowParts::V1(parts) => encode_v1(parts),
        VersionedWorkflowParts::V2(ir) => encode_v2(ir),
    }
}

/// Decodes one complete, bounded workflow after selecting its supported reader.
pub fn decode_compiled_ir(
    bytes: &[u8],
    outer_version: Option<SchemaVersion>,
) -> Result<CompiledWorkflow, JournalError> {
    checked_byte_length(bytes.len())?;
    let reader = match outer_version {
        Some(version) => outer_reader(version)?,
        None if bytes.starts_with(&V2_PREFIX) => Reader::V2,
        None => Reader::V1,
    };
    match reader {
        Reader::V1 if bytes.starts_with(&V2_PREFIX) => Err(JournalError::ArtifactMalformed),
        Reader::V1 => decode_v1(bytes),
        Reader::V2 => decode_v2(bytes),
    }
}

enum Reader {
    V1,
    V2,
}

fn version_error(
    held: SchemaVersion,
    required: SchemaVersion,
    class: CompatibilityClass,
) -> JournalError {
    JournalError::ArtifactVersionIncompatible {
        held,
        required,
        class,
    }
}

fn outer_reader(held: SchemaVersion) -> Result<Reader, JournalError> {
    let required = if held.major == 1 {
        CURRENT_IR_VERSION
    } else {
        V2_VERSION
    };
    let class = classify(artifact_ir_version_inputs_for(held, required)).class;
    if !class.authorizes_live_effect() {
        return Err(version_error(held, required, class));
    }
    match held.major {
        1 => Ok(Reader::V1),
        2 => Ok(Reader::V2),
        _ => Err(version_error(
            held,
            V2_VERSION,
            CompatibilityClass::Rejected,
        )),
    }
}

fn size_error(len: u32) -> JournalError {
    JournalError::PayloadTooLarge {
        len,
        max: MAX_COMPILED_IR_BYTES,
    }
}

fn checked_byte_length(length: usize) -> Result<usize, JournalError> {
    let len = u32::try_from(length).map_err(|_| size_error(u32::MAX))?;
    if len > MAX_COMPILED_IR_BYTES {
        return Err(size_error(len));
    }
    Ok(length)
}

fn wire_length<T: Serialize>(value: &T, prefix: &[u8]) -> Result<usize, JournalError> {
    let body = postcard::experimental::serialized_size(value)
        .map_err(JournalError::PostcardEncodeFailed)?;
    let total = prefix
        .len()
        .checked_add(body)
        .ok_or_else(|| size_error(u32::MAX))?;
    checked_byte_length(total)
}

fn encode_wire<T: Serialize>(value: &T, prefix: &[u8]) -> Result<Box<[u8]>, JournalError> {
    let length = wire_length(value, prefix)?;
    let mut wire = Vec::new();
    wire.try_reserve_exact(length)
        .map_err(JournalError::AdmissionAllocationFailed)?;
    wire.extend_from_slice(prefix);
    postcard::to_extend(value, wire)
        .map(Vec::into_boxed_slice)
        .map_err(JournalError::PostcardEncodeFailed)
}

fn decode_full<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, JournalError> {
    let (value, remaining) =
        postcard::take_from_bytes(bytes).map_err(JournalError::PostcardDecodeFailed)?;
    if !remaining.is_empty() {
        return Err(JournalError::ArtifactMalformed);
    }
    Ok(value)
}

fn canonical_v1(parts: &WorkflowParts) -> Result<Box<[u8]>, JournalError> {
    let mut preimage = parts.clone();
    preimage.digest = WorkflowDigest::from_bytes([0; 32]);
    encode_wire(&preimage, &[])
}

fn canonical_v2(ir: &WorkflowIrV2) -> Result<Box<[u8]>, JournalError> {
    let mut preimage = ir.clone();
    preimage.parts.digest = WorkflowDigest::from_bytes([0; 32]);
    encode_wire(&preimage, &V2_PREFIX)
}

fn verify_digest(preimage: &[u8], expected: WorkflowDigest) -> Result<(), JournalError> {
    let computed = WorkflowDigest::from_bytes(blake3::hash(preimage).into());
    if computed != expected {
        return Err(JournalError::ArtifactChecksumMismatch);
    }
    Ok(())
}

fn reconstruct_v2(ir: &WorkflowIrV2) -> Result<CompiledWorkflow, JournalError> {
    CompiledWorkflow::try_from_parts_with_symbols(ir.parts.clone(), ir.symbol_payloads.clone())
        .map_err(JournalError::WorkflowReconstruction)
}

fn encode_v1(parts: WorkflowParts) -> Result<Box<[u8]>, JournalError> {
    wire_length(&parts, &[])?;
    CompiledWorkflow::try_from_parts(parts.clone())
        .map_err(JournalError::WorkflowReconstruction)?;
    verify_digest(&canonical_v1(&parts)?, parts.digest)?;
    encode_wire(&parts, &[])
}

fn encode_v2(ir: WorkflowIrV2) -> Result<Box<[u8]>, JournalError> {
    wire_length(&ir, &V2_PREFIX)?;
    if ir.wire_version != 2 {
        return Err(JournalError::ArtifactMalformed);
    }
    reconstruct_v2(&ir)?;
    verify_digest(&canonical_v2(&ir)?, ir.parts.digest)?;
    encode_wire(&ir, &V2_PREFIX)
}

fn decode_v1(bytes: &[u8]) -> Result<CompiledWorkflow, JournalError> {
    let parts: WorkflowParts = decode_full(bytes)?;
    let workflow = CompiledWorkflow::try_from_parts(parts.clone())
        .map_err(JournalError::WorkflowReconstruction)?;
    verify_digest(&canonical_v1(&parts)?, parts.digest)?;
    Ok(workflow)
}

fn decode_v2(bytes: &[u8]) -> Result<CompiledWorkflow, JournalError> {
    let body = bytes
        .strip_prefix(&V2_PREFIX)
        .ok_or(JournalError::ArtifactMalformed)?;
    let ir: WorkflowIrV2 = decode_full(body)?;
    if ir.wire_version != 2 {
        return Err(JournalError::ArtifactMalformed);
    }
    let workflow = reconstruct_v2(&ir)?;
    verify_digest(&canonical_v2(&ir)?, ir.parts.digest)?;
    Ok(workflow)
}

#[cfg(test)]
#[path = "compiled_ir_tests.rs"]
mod tests;
