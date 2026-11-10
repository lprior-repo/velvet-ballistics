// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for accepted-envelope admission
// ============================================================================
//
// This file is a VERBATIM mirror of the prepared-proof shape
// production constructs for every admitted artifact. The mirror
// contains the same shape decision as production so any drift in
// field names, discriminant sets, or fn signatures breaks the
// companion `extern_accepted_envelope.rs` Verus build at compile
// time, which is the explicit drift-detection mechanism for the
// strict-envelope admission contract.
//
// Production sources mirrored:
//   - `vb_storage::admission::verification_proof_core`
//                                                 (crates/vb_storage/src/admission/proof.rs:103-118)
//   - `vb_storage::admission::prepare_with_profile` proof construction
//                                                 (crates/vb_storage/src/admission/verification.rs:39-42)
//   - `vb_storage::admission::verify_accepted_artifact` enforcement
//                                                 (crates/vb_storage/src/admission/verification.rs:60-90)
//   - `vb_storage::admission::verify_expected_bindings`
//                                                 (crates/vb_storage/src/admission/verification.rs:145-160)
//   - `vb_runtime::admission::verify_stored_artifact` error mapping
//                                                 (crates/vb_runtime/src/admission/registered_verification.rs:5-28)
//   - `vb_runtime::admission::ArtifactEnvelopeError`
//                                                 (crates/vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs:10-63)
//
// There is no production `is_strict_accepted` decision fn and no
// strict-policy gate validation in `submit_artifact_with_contracts`
// (crates/vb_storage/src/admission.rs:92-112 performs no gate-count
// or proof-flag check). The shape below is the decision production
// actually enforces, by recompute-and-compare, with every deviation
// rejected as `ArtifactEvidenceInvalid`.
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
// Production mirror: prepared-proof-shape decision
// ---------------------------------------------------------------------------
//
// Mirrors the prepared-proof shape production constructs for every
// admitted artifact (see the decision fn below). The projection to
// `Result<(), ArtifactEnvelopeErrorKind>` drops the I/O surface (no
// FjallJournal, no postcard, no blake3) and keeps only the pure
// decision.
//
// `REQUIRED_GATE_COUNT` is the verbatim mirror of
// `vb_runtime::admission::REQUIRED_GATE_COUNT = 15` at
// `crates/vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs:2`. The value `15` is
// inlined here to avoid depending on a separate type definition.
pub const REQUIRED_GATE_COUNT: u8 = 15;

// Production mirror of `vb_runtime::admission::ArtifactEnvelopeError`
// at `crates/vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs:10-63`,
// the full 12-variant discriminant set. Field-bearing variants are
// collapsed to bare discriminants because the spec only reasons about
// variant membership. `ArtifactEvidenceInvalid` is raised by independent
// runtime verification (`verify_stored_artifact` at
// `crates/vb_runtime/src/admission/registered_verification.rs:5-28`),
// not by the strict gate checks in `is_strict_accepted` below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactEnvelopeErrorKind {
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

// Prepared gate count: the gate_count `verification_proof_core`
// forces for every admitted artifact
// (crates/vb_storage/src/admission/proof.rs:103-118). `REQUIRED_GATE_COUNT`
// above mirrors the real `vb_runtime` constant (= 15) naming the
// legacy fixture count; it is NOT the count admission requires.
pub const PREPARED_GATE_COUNT: u8 = 0;

// ---------------------------------------------------------------------------
// Production decision fn: proof_matches_prepared_shape
// ---------------------------------------------------------------------------
//
// Mirrors the prepared-proof shape production constructs for every
// admitted artifact: `verification_proof_core` at
// `crates/vb_storage/src/admission/proof.rs:103-118` forces gate_count 0,
// durable false, and every proof flag false (supplied counts and
// durability are intentionally ignored by `VerificationProof::new` at
// admission/proof.rs:125-141), and `prepare_with_profile` at
// `crates/vb_storage/src/admission/verification.rs:39-42` then sets
// exactly `bounded_claimed` and `idempotency_verified_claimed` true
// and leaves the idempotency attestation empty.
// `submit_artifact_with_contracts` (admission.rs:92-112) performs no
// gate-count or proof-flag validation of its own; the shape is
// enforced by `verify_accepted_artifact` at
// `crates/vb_storage/src/admission/verification.rs:60-90`, which
// recomputes the expected artifact and rejects any deviation with
// `ArtifactEvidenceInvalid` (see the gate-count and proof-flag tests
// at `crates/vb_runtime/src/admission/artifact_envelope_tests.rs:205-318`).
//
// Returns `Ok(())` iff the proof carries exactly the prepared shape:
// gate_count 0, durable false, `bounded_claimed` and
// `idempotency_verified_claimed` true, `taint_safe_claimed`,
// `retry_safe_claimed`, and `replayable_claimed` false, the artifact
// digest equals the verification digest, and no idempotency
// attestation entries are present. Any deviation returns the single
// error production returns on this path, `ArtifactEvidenceInvalid`:
// production never returns the per-flag variants here, and never
// returns `InvalidGateCount` for a wrong gate count. A proof with the
// legacy 15-gate count and all flags true (the shape the
// always-present store fixture builds at
// `crates/vb_runtime/src/admission/parts/chunk_003_stores.rs:74-91`)
// does NOT match: it is not an admitted artifact, and the real
// validation rejects it.
//
// The projection to `Result<(), ArtifactEnvelopeErrorKind>` drops the
// I/O surface (no FjallJournal, no postcard, no blake3) and keeps only
// the pure decision.
//
// Marked `#[verifier::external]` so Verus skips body verification;
// the companion spec file attaches the production contract via
// `assume_specification[ production::proof_matches_prepared_shape ]`.
#[verifier::external]
pub fn proof_matches_prepared_shape(
    gate_count: u8,
    bounded_claimed: bool,
    taint_safe_claimed: bool,
    retry_safe_claimed: bool,
    durable: bool,
    replayable_claimed: bool,
    idempotency_verified_claimed: bool,
    artifact_digest_matches: bool,
    idempotency_attestation_absent: bool,
) -> Result<(), ArtifactEnvelopeErrorKind> {
    if gate_count != PREPARED_GATE_COUNT
        || !bounded_claimed
        || taint_safe_claimed
        || retry_safe_claimed
        || durable
        || replayable_claimed
        || !idempotency_verified_claimed
        || !artifact_digest_matches
        || !idempotency_attestation_absent
    {
        return Err(ArtifactEnvelopeErrorKind::ArtifactEvidenceInvalid);
    }
    Ok(())
}
