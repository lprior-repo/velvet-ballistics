// Verus verifier-only model for vb-core-cli-accepted-path PO-004.
// Trusted shell boundary: postcard decoding is represented as an input case.
//
// STRUCTURAL ALIGNMENT (verified against vb_runtime::admission):
//   ArtifactEnvelopeError variants (production):
//     ArtifactNotFound { digest }
//     PostcardDecodeFailed
//     InvalidGateCount { found, required }
//     MissingRequiredProofFlagBounded
//     MissingRequiredProofFlagTaintSafe
//     MissingRequiredProofFlagRetrySafe
//     MissingRequiredProofFlagDurable
//     MissingRequiredProofFlagReplayable
//     MissingRequiredProofFlagIdempotencyVerified
//     MissingIdempotencyAttestation { action }
//     ArtifactDigestMismatch { requested, found }
//   AdmissionError variants include all above mapped plus:
//     ArtifactNotFound, ArtifactEnvelopeDecodeFailed, ArtifactInvalidGateCount,
//     ArtifactInvalidProofFlag, ArtifactDigestMismatch,
//     CapabilityDenied, ResourceCapacityExceeded, BudgetPolicyExceeded,
//     ResourceBudgetOverflow, ResourceBudgetUnderflow, ResourceBudgetInvalidCapacity,
//     ResourceStepCeilingExceeded, ResourcePerTickCeilingExceeded,
//     ArtifactCertificateStale
//
// Spec ArtifactCase variants → production ArtifactEnvelopeError variants:
//   Missing          ↔ ArtifactNotFound { digest }
//   Malformed        ↔ PostcardDecodeFailed
//   InvalidProof     ↔ MissingRequiredProofFlag* (any proof flag variant)
//                        ∪ MissingIdempotencyAttestation { action }
//   InvalidGateCount ↔ InvalidGateCount { found, required }
//   InvalidCapability ↔ MissingRequiredProofFlagIdempotencyVerified
//                        (capability checks require idempotency attestation)
//   DigestMismatch   ↔ ArtifactDigestMismatch { requested, found }
//   Valid            ↔ (no error — artifact passed all checks)
//
// Spec AdmissionError variants → production AdmissionError variants:
//   NoError                          ↔ (none — admission succeeded)
//   StrictAdmissionMissingArtifact   ↔ ArtifactNotFound { digest }
//   MalformedAcceptedArtifact        ↔ ArtifactEnvelopeDecodeFailed
//   InvalidVerificationProof         ↔ ArtifactInvalidProofFlag { flag }
//   DigestMismatch                   ↔ ArtifactDigestMismatch { requested, found }
//
// NOTE: AdmissionError in production is #[non_exhaustive] with many additional
// variants (capability, budget, certificate-related) not present in the spec.
// The spec covers only the artifact-envelope admission path.

use vstd::prelude::*;

verus! {

/// Spec enum for artifact admission outcomes.
///
/// STRUCTURAL BINDING to vb_runtime::admission::ArtifactEnvelopeError:
///   Missing          ↔ ArtifactNotFound { digest }
///   Malformed        ↔ PostcardDecodeFailed
///   InvalidProof     ↔ MissingRequiredProofFlag* ∪ MissingIdempotencyAttestation
///   InvalidGateCount ↔ InvalidGateCount { found, required }
///   InvalidCapability ↔ MissingRequiredProofFlagIdempotencyVerified
///   DigestMismatch   ↔ ArtifactDigestMismatch { requested, found }
///   Valid            ↔ (no error)
pub enum ArtifactCase {
    Missing,
    Malformed,
    InvalidProof,
    InvalidGateCount,
    InvalidCapability,
    DigestMismatch,
    Valid,
}

/// Spec enum for admission error outcomes.
///
/// STRUCTURAL BINDING to vb_runtime::admission::AdmissionError:
///   NoError                          ↔ (none — admission succeeded)
///   StrictAdmissionMissingArtifact   ↔ ArtifactNotFound { digest }
///   MalformedAcceptedArtifact        ↔ ArtifactEnvelopeDecodeFailed
///   InvalidVerificationProof         ↔ ArtifactInvalidProofFlag { flag }
///   DigestMismatch                   ↔ ArtifactDigestMismatch { requested, found }
///
/// NOTE: Production AdmissionError is #[non_exhaustive] with additional
/// capability, budget, and certificate variants not in the spec scope.
pub enum AdmissionError {
    NoError,
    StrictAdmissionMissingArtifact,
    MalformedAcceptedArtifact,
    InvalidVerificationProof,
    DigestMismatch,
}

pub open spec fn outcome_error(case: ArtifactCase) -> AdmissionError {
    match case {
        ArtifactCase::Missing => AdmissionError::StrictAdmissionMissingArtifact,
        ArtifactCase::Malformed => AdmissionError::MalformedAcceptedArtifact,
        ArtifactCase::InvalidProof => AdmissionError::InvalidVerificationProof,
        ArtifactCase::InvalidGateCount => AdmissionError::InvalidVerificationProof,
        ArtifactCase::InvalidCapability => AdmissionError::InvalidVerificationProof,
        ArtifactCase::DigestMismatch => AdmissionError::DigestMismatch,
        ArtifactCase::Valid => AdmissionError::NoError,
    }
}

pub open spec fn outcome_admitted(case: ArtifactCase) -> bool {
    matches!(case, ArtifactCase::Valid)
}

pub open spec fn outcome_acknowledged(case: ArtifactCase) -> bool {
    outcome_admitted(case)
}

pub open spec fn outcome_run_state_inserted(case: ArtifactCase) -> bool {
    outcome_admitted(case)
}

pub open spec fn outcome_rejects(case: ArtifactCase) -> bool {
    !outcome_admitted(case)
}

pub open spec fn admission_outcome(case: ArtifactCase) -> (AdmissionError, bool, bool, bool) {
    (
        outcome_error(case),
        outcome_admitted(case),
        outcome_acknowledged(case),
        outcome_run_state_inserted(case),
    )
}

pub proof fn proof_valid_artifact_accepts_with_state()
    ensures
        outcome_admitted(ArtifactCase::Valid),
        outcome_acknowledged(ArtifactCase::Valid),
        outcome_run_state_inserted(ArtifactCase::Valid),
        outcome_error(ArtifactCase::Valid) == AdmissionError::NoError,
        !outcome_rejects(ArtifactCase::Valid),
        admission_outcome(ArtifactCase::Valid) == (AdmissionError::NoError, true, true, true),
{
}

pub proof fn proof_missing_rejects_before_ack()
    ensures
        outcome_rejects(ArtifactCase::Missing),
        outcome_error(ArtifactCase::Missing) == AdmissionError::StrictAdmissionMissingArtifact,
        !outcome_admitted(ArtifactCase::Missing),
        !outcome_acknowledged(ArtifactCase::Missing),
        !outcome_run_state_inserted(ArtifactCase::Missing),
{
}

pub proof fn proof_malformed_rejects_before_ack()
    ensures
        outcome_rejects(ArtifactCase::Malformed),
        outcome_error(ArtifactCase::Malformed) == AdmissionError::MalformedAcceptedArtifact,
        !outcome_admitted(ArtifactCase::Malformed),
        !outcome_acknowledged(ArtifactCase::Malformed),
        !outcome_run_state_inserted(ArtifactCase::Malformed),
{
}

pub proof fn proof_invalid_proof_rejects_before_ack()
    ensures
        outcome_rejects(ArtifactCase::InvalidProof),
        outcome_error(ArtifactCase::InvalidProof) == AdmissionError::InvalidVerificationProof,
        !outcome_admitted(ArtifactCase::InvalidProof),
        !outcome_acknowledged(ArtifactCase::InvalidProof),
        !outcome_run_state_inserted(ArtifactCase::InvalidProof),
{
}

pub proof fn proof_invalid_gate_count_rejects_before_ack()
    ensures
        outcome_rejects(ArtifactCase::InvalidGateCount),
        outcome_error(ArtifactCase::InvalidGateCount) == AdmissionError::InvalidVerificationProof,
        !outcome_admitted(ArtifactCase::InvalidGateCount),
        !outcome_acknowledged(ArtifactCase::InvalidGateCount),
        !outcome_run_state_inserted(ArtifactCase::InvalidGateCount),
{
}

pub proof fn proof_invalid_capability_rejects_before_ack()
    ensures
        outcome_rejects(ArtifactCase::InvalidCapability),
        outcome_error(ArtifactCase::InvalidCapability) == AdmissionError::InvalidVerificationProof,
        !outcome_admitted(ArtifactCase::InvalidCapability),
        !outcome_acknowledged(ArtifactCase::InvalidCapability),
        !outcome_run_state_inserted(ArtifactCase::InvalidCapability),
{
}

pub proof fn proof_digest_mismatch_rejects_before_ack()
    ensures
        outcome_rejects(ArtifactCase::DigestMismatch),
        outcome_error(ArtifactCase::DigestMismatch) == AdmissionError::DigestMismatch,
        !outcome_admitted(ArtifactCase::DigestMismatch),
        !outcome_acknowledged(ArtifactCase::DigestMismatch),
        !outcome_run_state_inserted(ArtifactCase::DigestMismatch),
{
}

pub proof fn proof_decision_total(case: ArtifactCase)
    ensures outcome_admitted(case) || outcome_rejects(case),
{
}

pub proof fn proof_rejection_before_ack_and_run_state(case: ArtifactCase)
    requires outcome_rejects(case),
    ensures
        !outcome_admitted(case),
        !outcome_acknowledged(case),
        !outcome_run_state_inserted(case),
        outcome_error(case) != AdmissionError::NoError,
{
}

pub proof fn proof_admission_possible_only_for_valid(case: ArtifactCase)
    requires outcome_admitted(case),
    ensures
        case == ArtifactCase::Valid,
        outcome_acknowledged(case),
        outcome_run_state_inserted(case),
        outcome_error(case) == AdmissionError::NoError,
{
}

} // verus!

fn main() {}
