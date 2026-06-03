#![forbid(unsafe_code)]
//! Artifact envelope validation for runtime admission.

use vb_core::ids::ActionId;

use super::errors::ArtifactEnvelopeError;
use super::types::REQUIRED_GATE_COUNT;

/// Validates an accepted artifact envelope.
///
/// Checks digest binding, gate count, proof flags, and idempotency attestations.
pub fn validate_accepted_artifact_envelope(
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> Result<(), ArtifactEnvelopeError> {
    if artifact.verification.digest != artifact.digest {
        return Err(ArtifactEnvelopeError::ArtifactDigestMismatch {
            requested: artifact.digest,
            found: artifact.verification.digest,
        });
    }
    if artifact.verification.gate_count != REQUIRED_GATE_COUNT {
        return Err(ArtifactEnvelopeError::InvalidGateCount {
            found: artifact.verification.gate_count,
            required: REQUIRED_GATE_COUNT,
        });
    }
    if !artifact.verification.bounded_claimed {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagBounded);
    }
    if !artifact.verification.taint_safe_claimed {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagTaintSafe);
    }
    if !artifact.verification.retry_safe_claimed {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagRetrySafe);
    }
    if !artifact.verification.durable {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagDurable);
    }
    if !artifact.verification.replayable_claimed {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagReplayable);
    }
    if !artifact.verification.idempotency_verified_claimed {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagIdempotencyVerified);
    }
    first_missing_idempotency_attestation(artifact).map_or(Ok(()), |action| {
        Err(ArtifactEnvelopeError::MissingIdempotencyAttestation { action })
    })
}

fn first_missing_idempotency_attestation(
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> Option<ActionId> {
    artifact
        .verification
        .idempotency_keyed
        .iter()
        .copied()
        .find(|action| !artifact.verification.idempotency_attested.contains(action))
}
