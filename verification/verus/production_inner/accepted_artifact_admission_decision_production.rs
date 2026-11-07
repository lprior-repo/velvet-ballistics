// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for accepted-artifact admission decision
// ============================================================================
//
// This file is a VERBATIM mirror of the production artifact envelope
// error taxonomy and admission decision types.
//
// Production sources mirrored:
//   - `vb_runtime::admission::ArtifactEnvelopeError` (crates/vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs:10-63)
//   - `vb_runtime::admission::map_artifact_envelope_error` (crates/vb_runtime/src/admission/parts/chunk_004_validation.rs:41-82)
//   - `vb_runtime::admission::admit_artifact_run_with_certificate_floor` strict branch
//     (crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:83-189)
//
// DRIFT POLICY: `crates/vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs:10-63`,
// `crates/vb_runtime/src/admission/parts/chunk_004_validation.rs:41-82`,
// `crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:83-189`
// Regenerate this file whenever production changes. Any new variant
// added to `ArtifactEnvelopeError` or any signature change in the
// strict-branch decision surface breaks the `extern_*` Verus build.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

#[derive(Clone, Copy)]
pub enum SpecArtifactEnvelopeError {
    ArtifactNotFound,
    PostcardDecodeFailed,
    ArtifactEvidenceInvalid,
    InvalidGateCount,
    MissingRequiredProofFlagBounded,
    MissingRequiredProofFlagTaintSafe,
    MissingRequiredProofFlagRetrySafe,
    MissingRequiredProofFlagDurable,
    MissingRequiredProofFlagReplayable,
    MissingRequiredProofFlagIdempotencyVerified,
    MissingIdempotencyAttestation,
    ArtifactDigestMismatch,
}

#[derive(Clone, Copy)]
pub enum SpecAdmissionError {
    NoError,
    StrictAdmissionMissingArtifact,
    MalformedAcceptedArtifact,
    InvalidVerificationProof,
    DigestMismatch,
}

#[derive(Clone, Copy)]
pub struct SpecAdmissionOutcome {
    pub error: SpecAdmissionError,
    pub admitted: bool,
    pub acknowledged: bool,
    pub run_state_inserted: bool,
}

#[verifier::external]
pub fn admission_decision(err: SpecArtifactEnvelopeError) -> SpecAdmissionOutcome {
    SpecAdmissionOutcome {
        error: map_to_spec_error(err),
        admitted: false,
        acknowledged: false,
        run_state_inserted: false,
    }
}

#[verifier::external]
pub fn admission_decision_ok() -> SpecAdmissionOutcome {
    SpecAdmissionOutcome {
        error: SpecAdmissionError::NoError,
        admitted: true,
        acknowledged: true,
        run_state_inserted: true,
    }
}

// Production dispatch table: collapses the 12 production
// `ArtifactEnvelopeError` variants from
// `vb_runtime::admission::map_artifact_envelope_error`
// (crates/vb_runtime/src/admission/parts/chunk_004_validation.rs:41-82) into the 5-class
// `SpecAdmissionError` lattice used by the spec predicate
// `spec_outcome_error` at
// `verification/verus/accepted_artifact_admission_decision.rs:106-141`.
//
// The 1-to-many collapse rule (production -> spec):
//   - `ArtifactNotFound`            -> `StrictAdmissionMissingArtifact`
//   - `PostcardDecodeFailed`        -> `MalformedAcceptedArtifact`
//   - `ArtifactEvidenceInvalid`     -> `InvalidVerificationProof`
//   - `InvalidGateCount`            -> `InvalidVerificationProof`
//   - 6x `MissingRequiredProofFlag*`-> `InvalidVerificationProof`
//   - `MissingIdempotencyAttestation` -> `InvalidVerificationProof`
//   - `ArtifactDigestMismatch`     -> `DigestMismatch`
#[verifier::external]
fn map_to_spec_error(err: SpecArtifactEnvelopeError) -> SpecAdmissionError {
    match err {
        SpecArtifactEnvelopeError::ArtifactNotFound => {
            SpecAdmissionError::StrictAdmissionMissingArtifact
        }
        SpecArtifactEnvelopeError::PostcardDecodeFailed => {
            SpecAdmissionError::MalformedAcceptedArtifact
        }
        SpecArtifactEnvelopeError::ArtifactEvidenceInvalid => {
            SpecAdmissionError::InvalidVerificationProof
        }
        SpecArtifactEnvelopeError::InvalidGateCount => SpecAdmissionError::InvalidVerificationProof,
        SpecArtifactEnvelopeError::MissingRequiredProofFlagBounded => {
            SpecAdmissionError::InvalidVerificationProof
        }
        SpecArtifactEnvelopeError::MissingRequiredProofFlagTaintSafe => {
            SpecAdmissionError::InvalidVerificationProof
        }
        SpecArtifactEnvelopeError::MissingRequiredProofFlagRetrySafe => {
            SpecAdmissionError::InvalidVerificationProof
        }
        SpecArtifactEnvelopeError::MissingRequiredProofFlagDurable => {
            SpecAdmissionError::InvalidVerificationProof
        }
        SpecArtifactEnvelopeError::MissingRequiredProofFlagReplayable => {
            SpecAdmissionError::InvalidVerificationProof
        }
        SpecArtifactEnvelopeError::MissingRequiredProofFlagIdempotencyVerified => {
            SpecAdmissionError::InvalidVerificationProof
        }
        SpecArtifactEnvelopeError::MissingIdempotencyAttestation => {
            SpecAdmissionError::InvalidVerificationProof
        }
        SpecArtifactEnvelopeError::ArtifactDigestMismatch => SpecAdmissionError::DigestMismatch,
    }
}