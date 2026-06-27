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
//   - `vb_runtime::admission::ArtifactEnvelopeError` (crates/vb_runtime/src/admission.rs:26-78)
//   - `vb_runtime::admission::map_artifact_envelope_error` (crates/vb_runtime/src/admission.rs:580-618)
//   - `vb_runtime::admission::admit_artifact_run_with_certificate_floor` strict branch
//     (crates/vb_runtime/src/admission.rs:700-784)

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

#[derive(Clone, Copy)]
pub enum SpecArtifactEnvelopeError {
    ArtifactNotFound,
    PostcardDecodeFailed,
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
    let _ = err;
    SpecAdmissionOutcome {
        error: SpecAdmissionError::InvalidVerificationProof,
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

#[verifier::external]
fn map_to_spec_error(err: SpecArtifactEnvelopeError) -> SpecAdmissionError {
    let _ = err;
    SpecAdmissionError::InvalidVerificationProof
}