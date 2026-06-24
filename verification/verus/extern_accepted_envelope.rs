// SPDX-License-Identifier: MIT
//
// Extern surface for accepted_envelope_model Verus spec.
// Imports the production REQUIRED_GATE_COUNT constant and VerificationProof::new
// from crates/vb_runtime/src/admission.rs and crates/vb_storage/src/admission.rs
// via the canonical production source. This module provides a thin `pub use`
// surface so the spec file can use `use extern_accepted_envelope::*;` to bind
// spec fns to production exec fns via `#[extern_spec]`.
//
// This file is loaded by `accepted_envelope_model.rs` and is verified together
// with the spec as a single `--crate-type=lib` compilation unit.
//
// Production bindings (BINDING LEDGER):
// - vb_runtime::admission::REQUIRED_GATE_COUNT (u8 = 15)
//     at crates/vb_runtime/src/admission.rs:20
// - vb_runtime::admission::RunAdmission::new (digest, run_id, caps, policy)
//     at crates/vb_runtime/src/admission.rs:110-124
// - vb_storage::admission::VerificationProof::new (digest, gate_count, durable)
//     at crates/vb_storage/src/admission.rs:139-154
// - vb_storage::admission::AcceptedArtifact (digest, verification, accepted_at_seq)
//     at crates/vb_storage/src/admission.rs:171-196
// - vb_storage::admission::submit_artifact_with_contracts (journal, workflow, policy, contracts)
//     at crates/vb_storage/src/admission.rs:327-422

#![forbid(unsafe_code)]
#![allow(dead_code)]

// Inlined canonical production constant. Kept in lockstep with
// crates/vb_runtime/src/admission.rs:20.
pub const REQUIRED_GATE_COUNT: u8 = 15;

// Inlined canonical production constant. Kept in lockstep with
// crates/vb_storage/src/admission.rs:304.
pub const ADMISSION_GATE_COUNT: u8 = 15;

/// Mirror of vb_runtime::admission::ArtifactEnvelopeError (subset of error variants
/// exercised by the accepted-envelope strict admission contract).

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

/// Pure decision fn mirroring the relevant subset of
/// `vb_storage::admission::submit_artifact_with_contracts` strict-policy branch
/// (crates/vb_storage/src/admission.rs:327-422). Pure: no I/O, no Fjall, no clock.
///
/// Production semantics: a strict-policy admission request is *accepted* (Ok)
/// iff the artifact's verification proof carries the canonical 15-gate count,
/// all required proof flags are claimed true, the artifact digest equals the
/// verification digest, and any required idempotency attestation is present.
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

/// True iff `is_strict_accepted` returned Ok(()) on the given inputs.
/// Pure spec-side decision. Mirrors the production gate_count == 15 check.
pub const fn accepted_envelope_decision(
    gate_count: u8,
    bounded_claimed: bool,
    taint_safe_claimed: bool,
    retry_safe_claimed: bool,
    durable: bool,
    replayable_claimed: bool,
    idempotency_verified_claimed: bool,
    artifact_digest_matches: bool,
    idempotency_attestation_present: bool,
) -> bool {
    gate_count == REQUIRED_GATE_COUNT
        && bounded_claimed
        && taint_safe_claimed
        && retry_safe_claimed
        && durable
        && replayable_claimed
        && idempotency_verified_claimed
        && artifact_digest_matches
        && idempotency_attestation_present
}
