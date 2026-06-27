// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for accepted-envelope admission
// ============================================================================
//
// This file is a VERBATIM mirror of the production `is_strict_accepted`
// decision logic. The mirror contains the same gate-validation semantics
// as the production `submit_artifact_with_contracts` strict-policy
// branch so any drift in field names, discriminant sets, or fn
// signatures breaks the companion `extern_accepted_envelope.rs` Verus
// build at compile time, which is the explicit drift-detection
// mechanism for the strict-envelope admission contract.
//
// Production source mirrored:
//   - `vb_storage::admission::submit_artifact_with_contracts` strict
//     branch (crates/vb_storage/src/admission.rs:327-422), projected
//     onto the pure decision shape used by the spec.
//
// DRIFT POLICY: This file MUST be regenerated whenever the production
// source changes. The mirror is annotated with the originating
// production line ranges so regeneration is mechanical. The companion
// extern file `extern_accepted_envelope.rs` includes this file via
// `#[path = "production_inner/accepted_envelope_production.rs"]`.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// Production mirror: submit_artifact_with_contracts strict branch decision
// ---------------------------------------------------------------------------
//
// Mirrors the strict-policy decision in
// `vb_storage::admission::submit_artifact_with_contracts` at
// `crates/vb_storage/src/admission.rs:412-415`. Returns Ok(()) iff
// the artifact's verification proof carries the canonical 15-gate
// count, all required proof flags are claimed true, the artifact
// digest equals the verification digest, and any required idempotency
// attestation is present.
//
// The body is a verbatim logic copy of the production gate validation
// branch (lines 412-415 plus the earlier proof-flag checks at lines
// 119-128). The projection to `Result<(), ArtifactEnvelopeErrorKind>`
// drops the I/O surface (no FjallJournal, no postcard, no blake3) and
// keeps only the pure decision.
//
// `REQUIRED_GATE_COUNT` is the verbatim mirror of
// `vb_runtime::admission::REQUIRED_GATE_COUNT = 15` at
// `crates/vb_runtime/src/admission.rs:20`. The value `15` is
// inlined here to avoid depending on a separate type definition.
pub const REQUIRED_GATE_COUNT: u8 = 15;

// Production mirror of `vb_runtime::admission::ArtifactEnvelopeError`
// at `crates/vb_runtime/src/admission.rs:22-79`, restricted to the
// discriminant variants exercised by the strict-policy branch of
// `submit_artifact_with_contracts`. Field-bearing variants are
// collapsed to bare discriminants because the spec only reasons about
// variant membership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactEnvelopeErrorKind {
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

// Production decision fn: the strict-admission gate validation. Mirrors
// the strict-policy branch of `submit_artifact_with_contracts` at
// `crates/vb_storage/src/admission.rs:412-415`.
//
// Body: verbatim copy of the production gate validation, with
// references to `REQUIRED_GATE_COUNT` and `ArtifactEnvelopeErrorKind`
// inlined so the body is self-contained.
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
) -> Result<(), ArtifactEnvelopeErrorKind> {
    if gate_count != REQUIRED_GATE_COUNT {
        return Err(ArtifactEnvelopeErrorKind::InvalidGateCount);
    }
    if !bounded_claimed {
        return Err(ArtifactEnvelopeErrorKind::MissingRequiredProofFlagBounded);
    }
    if !taint_safe_claimed {
        return Err(ArtifactEnvelopeErrorKind::MissingRequiredProofFlagTaintSafe);
    }
    if !retry_safe_claimed {
        return Err(ArtifactEnvelopeErrorKind::MissingRequiredProofFlagRetrySafe);
    }
    if !durable {
        return Err(ArtifactEnvelopeErrorKind::MissingRequiredProofFlagDurable);
    }
    if !replayable_claimed {
        return Err(ArtifactEnvelopeErrorKind::MissingRequiredProofFlagReplayable);
    }
    if !idempotency_verified_claimed {
        return Err(ArtifactEnvelopeErrorKind::MissingRequiredProofFlagIdempotencyVerified);
    }
    if !artifact_digest_matches {
        return Err(ArtifactEnvelopeErrorKind::ArtifactDigestMismatch);
    }
    if !idempotency_attestation_present {
        return Err(ArtifactEnvelopeErrorKind::MissingIdempotencyAttestation);
    }
    Ok(())
}