// SPDX-License-Identifier: MIT
//
// Extern surface for accepted_envelope_model Verus spec.
// ============================================================================
//
// Production binding (BINDING LEDGER):
//   - vb_runtime::admission::REQUIRED_GATE_COUNT (u8 = 15)
//     at crates/vb_runtime/src/admission.rs:20
//   - vb_storage::admission::submit_artifact_with_contracts strict-policy
//     branch (crates/vb_storage/src/admission.rs:327-422)
//
// The `#[path]` import below binds this spec file to a thin in-tree
// `production_inner/accepted_envelope_production.rs` mirror that exposes
// the verbatim production `REQUIRED_GATE_COUNT`, the discriminant
// variants of `ArtifactEnvelopeError`, and the pure `is_strict_accepted`
// decision fn whose semantics match the strict-policy branch of the
// production `submit_artifact_with_contracts`. The spec file attaches
// `assume_specification` to that production decision fn, and each proof
// fn non-vacuously proves a different structural property of the spec.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION INCLUSION via #[path] — STRUCTURAL drift detection
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of
// verification/verus/production_inner/accepted_envelope_production.rs.
// The mirror is marked `#[verifier::external]` at module level so the
// production bodies are opaque to Verus; the inclusion still validates
// Rust resolution (field names, discriminant sets, fn signatures) at
// compile time. Any drift in the production impl surface breaks this
// Verus build.
//
// The `prod` module exposes `REQUIRED_GATE_COUNT`,
// `ArtifactEnvelopeErrorKind`, and `is_strict_accepted` to the wrapper
// functions in this file. Inside `prod`, these items are opaque
// (`#[verifier::external]` at module level).
#[verifier::external]
#[path = "production_inner/accepted_envelope_production.rs"]
pub mod prod;

// ---------------------------------------------------------------------------
// Verus-visible error type — used by `assume_specification`
// ---------------------------------------------------------------------------
//
// `SpecArtifactEnvelopeError` mirrors the production
// `ArtifactEnvelopeErrorKind` discriminant variants. The spec uses
// this Verus-native type in `Result<..., SpecArtifactEnvelopeError>`
// positions; the wrapper below converts from the production type
// (which is opaque to Verus) to this Verus-visible type.

#[derive(Debug, Clone, PartialEq, Eq)]
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

// Conversion: production error discriminant -> Verus-visible error
// discriminant. Both enums share the same discriminant set; this
// function performs a direct mapping.
#[verifier::external]
fn convert_artifact_envelope_error(e: prod::ArtifactEnvelopeErrorKind) -> SpecArtifactEnvelopeError {
    match e {
        prod::ArtifactEnvelopeErrorKind::ArtifactNotFound => SpecArtifactEnvelopeError::ArtifactNotFound,
        prod::ArtifactEnvelopeErrorKind::PostcardDecodeFailed => SpecArtifactEnvelopeError::PostcardDecodeFailed,
        prod::ArtifactEnvelopeErrorKind::InvalidGateCount => SpecArtifactEnvelopeError::InvalidGateCount,
        prod::ArtifactEnvelopeErrorKind::MissingRequiredProofFlagBounded => SpecArtifactEnvelopeError::MissingRequiredProofFlagBounded,
        prod::ArtifactEnvelopeErrorKind::MissingRequiredProofFlagTaintSafe => SpecArtifactEnvelopeError::MissingRequiredProofFlagTaintSafe,
        prod::ArtifactEnvelopeErrorKind::MissingRequiredProofFlagRetrySafe => SpecArtifactEnvelopeError::MissingRequiredProofFlagRetrySafe,
        prod::ArtifactEnvelopeErrorKind::MissingRequiredProofFlagDurable => SpecArtifactEnvelopeError::MissingRequiredProofFlagDurable,
        prod::ArtifactEnvelopeErrorKind::MissingRequiredProofFlagReplayable => SpecArtifactEnvelopeError::MissingRequiredProofFlagReplayable,
        prod::ArtifactEnvelopeErrorKind::MissingRequiredProofFlagIdempotencyVerified => SpecArtifactEnvelopeError::MissingRequiredProofFlagIdempotencyVerified,
        prod::ArtifactEnvelopeErrorKind::MissingIdempotencyAttestation => SpecArtifactEnvelopeError::MissingIdempotencyAttestation,
        prod::ArtifactEnvelopeErrorKind::ArtifactDigestMismatch => SpecArtifactEnvelopeError::ArtifactDigestMismatch,
    }
}

// Production exec fn: `is_strict_accepted` (the non-opaque wrapper).
// The body calls the production `is_strict_accepted` (via the prod
// mirror) and converts the error discriminant. The body is opaque to
// Verus (`#[verifier::external]`); the spec file attaches the
// production contract via `assume_specification`.
#[verifier::external]
pub fn is_strict_accepted(
    gate_count: u8,
    bounded_claimed: bool,
    taint_safe_claimed: bool,
    retry_safe_claimed: bool,
    durable: bool,
    replayable_claimed: bool,
    idempotency_verified_claimed: bool,
    artifact_digest_matches: bool,
    idempotency_attestation_present: bool,
) -> Result<(), SpecArtifactEnvelopeError> {
    prod::is_strict_accepted(
        gate_count,
        bounded_claimed,
        taint_safe_claimed,
        retry_safe_claimed,
        durable,
        replayable_claimed,
        idempotency_verified_claimed,
        artifact_digest_matches,
        idempotency_attestation_present,
    )
    .map_err(convert_artifact_envelope_error)
}

} // verus!