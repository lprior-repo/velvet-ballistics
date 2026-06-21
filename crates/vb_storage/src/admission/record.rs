#![forbid(unsafe_code)]
//! Record validation and deserialization for accepted-artifact envelopes.

use crate::codec::payload;
use crate::error::JournalError;
use crate::records::CompiledIrRecord;

use super::policy::is_accepted_gate_count;
use super::types::{AcceptedArtifact, VerificationProof};

const MIN_ACCEPTED_ARTIFACT_ENVELOPE_BYTES: usize = 96;

/// Accepted-artifact envelope length precheck before postcard decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AcceptedArtifactEnvelopeLengthDecision {
    /// Input cannot contain the three fixed digest fields required by v1.
    TooShort,
    /// Input is long enough to attempt canonical postcard decoding.
    Decode,
}

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
    validate_accepted_artifact_digest(&artifact, record.digest)?;
    validate_artifact_metadata_hash_binding(record, &artifact)
}

pub fn decode_accepted_artifact_envelope(bytes: &[u8]) -> Result<AcceptedArtifact, JournalError> {
    if classify_accepted_artifact_envelope_len(bytes.len())
        == AcceptedArtifactEnvelopeLengthDecision::TooShort
    {
        return Err(JournalError::ArtifactMalformed);
    }
    let (artifact, remaining) =
        postcard::take_from_bytes(bytes).map_err(|_| JournalError::ArtifactMalformed)?;
    let declared_end = bytes
        .len()
        .checked_sub(remaining.len())
        .ok_or(JournalError::UnexpectedEof)?;
    payload::reject_trailing_bytes(declared_end, bytes.len())?;
    Ok(artifact)
}

pub(crate) fn classify_accepted_artifact_envelope_len(
    len: usize,
) -> AcceptedArtifactEnvelopeLengthDecision {
    if len < MIN_ACCEPTED_ARTIFACT_ENVELOPE_BYTES {
        AcceptedArtifactEnvelopeLengthDecision::TooShort
    } else {
        AcceptedArtifactEnvelopeLengthDecision::Decode
    }
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

/// Defense-in-depth check that the stored `metadata_hash` matches the
/// metadata hash recomputed from the artifact bytes. A `None` value is
/// accepted for backward compatibility with pre-mutation-protection
/// records, but a `Some(stored)` value must agree with the recomputed
/// hash; a mismatch indicates same-digest metadata mutation and is
/// rejected with [`JournalError::MetadataMutation`].
pub(crate) fn validate_artifact_metadata_hash_binding(
    record: &crate::records::CompiledIrRecord,
    artifact: &AcceptedArtifact,
) -> Result<(), JournalError> {
    let stored = match record.metadata_hash {
        Some(stored) => stored,
        None => return Ok(()),
    };
    let computed = super::metadata::compute_artifact_metadata_hash(artifact);
    if stored == computed {
        Ok(())
    } else {
        Err(JournalError::MetadataMutation {
            digest: record.digest,
        })
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
    if proof.gate_count == 0 {
        if proof.durable || has_any_proof_flag(proof) {
            return Err(JournalError::ArtifactMalformed);
        }
    } else if let Some(flag) = missing_proof_flag(proof) {
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

fn has_any_proof_flag(proof: &VerificationProof) -> bool {
    proof.bounded_claimed
        || proof.taint_safe_claimed
        || proof.retry_safe_claimed
        || proof.idempotency_verified_claimed
        || proof.replayable_claimed
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MissingProofFlag {
    Bounded,
    TaintSafe,
    RetrySafe,
    IdempotencyVerified,
    Replayable,
}

impl MissingProofFlag {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Bounded => "bounded",
            Self::TaintSafe => "taint_safe",
            Self::RetrySafe => "retry_safe",
            Self::IdempotencyVerified => "idempotency_verified",
            Self::Replayable => "replayable",
        }
    }
}

pub(crate) fn missing_proof_flag(proof: &VerificationProof) -> Option<&'static str> {
    match missing_proof_flag_kind(proof) {
        Some(flag) => Some(flag.as_str()),
        None => None,
    }
}

pub(crate) fn missing_proof_flag_kind(proof: &VerificationProof) -> Option<MissingProofFlag> {
    if !proof.bounded_claimed {
        Some(MissingProofFlag::Bounded)
    } else if !proof.taint_safe_claimed {
        Some(MissingProofFlag::TaintSafe)
    } else if !proof.retry_safe_claimed {
        Some(MissingProofFlag::RetrySafe)
    } else if !proof.idempotency_verified_claimed {
        Some(MissingProofFlag::IdempotencyVerified)
    } else if !proof.replayable_claimed {
        Some(MissingProofFlag::Replayable)
    } else {
        None
    }
}
