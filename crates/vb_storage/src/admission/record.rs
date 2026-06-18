#![forbid(unsafe_code)]
//! Record validation and deserialization for accepted-artifact envelopes.

use crate::codec::payload;
use crate::error::JournalError;
use crate::records::CompiledIrRecord;
use crate::types::EventSeq;

use super::policy::{ADMISSION_GATE_COUNT, is_accepted_gate_count};
use super::types::{AcceptedArtifact, VerificationProof};

/// Validates and persists a compiled workflow artifact.
///
/// Structure validation ensures the workflow can be reconstructed from its parts.
/// Checksum validation recomputes the BLAKE3 digest from the serialized parts
/// and compares it to the digest claimed by the workflow.
///
/// On success, the artifact is stored in the `compiled_ir` keyspace and its
/// digest is returned. On failure, the storage is left unchanged.
pub fn admit_compiled_artifact(
    journal: &crate::journal::FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_core::WorkflowDigest, JournalError> {
    let artifact =
        super::flow::submit_artifact(journal, workflow, vb_core::RuntimePolicy::Journaled)?;
    Ok(artifact.digest)
}

/// Validates a stored compiled-IR record and rejects malformed accepted-artifact envelopes.
pub fn validate_compiled_ir_record(record: &CompiledIrRecord) -> Result<(), JournalError> {
    super::bytes::reject_oversized_compiled_ir_value(record.ir.len())?;
    let artifact = decode_accepted_artifact_envelope(&record.ir)?;
    validate_accepted_artifact_digest(&artifact, record.digest)
}

pub fn decode_accepted_artifact_envelope(bytes: &[u8]) -> Result<AcceptedArtifact, JournalError> {
    let (artifact, remaining) =
        postcard::take_from_bytes(bytes).map_err(|_| JournalError::ArtifactMalformed)?;
    let declared_end = bytes
        .len()
        .checked_sub(remaining.len())
        .ok_or(JournalError::UnexpectedEof)?;
    payload::reject_trailing_bytes(declared_end, bytes.len())?;
    Ok(artifact)
}

pub(crate) fn validate_accepted_artifact_digest(
    artifact: &AcceptedArtifact,
    digest: vb_core::WorkflowDigest,
) -> Result<(), JournalError> {
    validate_accepted_artifact_metadata(artifact)?;
    if artifact.digest != digest || artifact.verification.digest != digest {
        return Err(JournalError::ArtifactChecksumMismatch);
    }
    Ok(())
}

pub(crate) fn validate_accepted_artifact_metadata(
    artifact: &AcceptedArtifact,
) -> Result<(), JournalError> {
    if artifact.source_digest != artifact.digest {
        return Err(JournalError::ArtifactMalformed);
    }
    validate_artifact_policy_digest(artifact)?;
    validate_verification_proof(&artifact.verification)
}

pub(crate) fn validate_artifact_policy_digest(
    artifact: &AcceptedArtifact,
) -> Result<(), JournalError> {
    let workflow = workflow_from_artifact_ir(artifact)?;
    if artifact.policy_digest == super::policy::compute_policy_digest(&workflow)? {
        Ok(())
    } else {
        Err(JournalError::ArtifactMalformed)
    }
}

fn workflow_from_artifact_ir(
    artifact: &AcceptedArtifact,
) -> Result<vb_core::CompiledWorkflow, JournalError> {
    let (mut parts, remaining) = postcard::take_from_bytes::<vb_core::WorkflowParts>(&artifact.ir)
        .map_err(|_| JournalError::ArtifactMalformed)?;
    let declared_end = artifact
        .ir
        .len()
        .checked_sub(remaining.len())
        .ok_or(JournalError::UnexpectedEof)?;
    payload::reject_trailing_bytes(declared_end, artifact.ir.len())?;
    parts.digest = artifact.digest;
    vb_core::CompiledWorkflow::try_from_parts(parts).map_err(|_| JournalError::ArtifactMalformed)
}

fn validate_verification_proof(proof: &VerificationProof) -> Result<(), JournalError> {
    if !is_accepted_gate_count(proof.gate_count) {
        return Err(JournalError::InvalidGateCount {
            found: proof.gate_count,
        });
    }
    if proof.gate_count == 0 && proof.durable {
        return Err(JournalError::ArtifactMalformed);
    }
    if let Some(flag) = missing_proof_flag(proof) {
        return Err(JournalError::MissingRequiredProofFlag { flag });
    }
    if !proof
        .warnings
        .iter()
        .all(super::types::VerificationWarning::is_valid)
    {
        return Err(JournalError::ArtifactMalformed);
    }
    Ok(())
}

pub(crate) fn missing_proof_flag(proof: &VerificationProof) -> Option<&'static str> {
    if !proof.bounded_claimed {
        Some("bounded")
    } else if !proof.taint_safe_claimed {
        Some("taint_safe")
    } else if !proof.retry_safe_claimed {
        Some("retry_safe")
    } else if !proof.idempotency_verified_claimed {
        Some("idempotency_verified")
    } else if !proof.replayable_claimed {
        Some("replayable")
    } else {
        None
    }
}
