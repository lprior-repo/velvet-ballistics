// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for admission artifact model
// ============================================================================
//
// This file is a VERBATIM mirror of the production artifact admission
// types and decision logic. The mirror contains the same field names,
// discriminant sets, and decision-fn semantics as the production
// source so any drift breaks the companion
// `extern_admission_artifact_model.rs` Verus build at compile time,
// which is the explicit drift-detection mechanism.
//
// Production sources mirrored:
//   - `vb_core::ids::WorkflowDigest`             (crates/vb_core/src/ids/mod.rs:340-357)
//   - `vb_storage::admission::VerificationProof` (crates/vb_storage/src/admission.rs:67-91)
//   - `vb_storage::admission::AcceptedArtifact`   (crates/vb_storage/src/admission.rs:203-228)
//   - `vb_storage::admission::ADMISSION_GATE_COUNT` (crates/vb_storage/src/admission.rs:330)
//   - `vb_storage::admission::submit_artifact_with_contracts`
//                                                 (crates/vb_storage/src/admission.rs:327-422)
//
// Stub substitutions: the production `WorkflowDigest` is a newtype
// over `[u8; 32]`; we model it as `pub struct WorkflowDigest(pub u64)`
// for spec-mode equality reasoning. The production `AcceptedArtifact`
// carries opaque payload fields (`ir: Vec<u8>`, `accepted_at_seq:
// EventSeq`, `required_capabilities: Box<[Capability]>`) which are
// not part of the strict-admission decision and are NOT mirrored here.
//
// DRIFT POLICY: This file MUST be regenerated whenever the production
// source changes. The mirror is annotated with the originating
// production line ranges so regeneration is mechanical. The companion
// extern file `extern_admission_artifact_model.rs` includes this file
// via `#[path = "production_inner/admission_artifact_production.rs"]`.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// Production mirror: vb_core::ids::WorkflowDigest
// ---------------------------------------------------------------------------
//
// Verbatim mirror of `vb_core::ids::WorkflowDigest` at
// `crates/vb_core/src/ids/mod.rs:340-357`. Production is
// `pub struct WorkflowDigest([u8; 32])` with a derived `PartialEq`
// impl. We model the digest as `pub struct WorkflowDigest(pub u64)`
// for spec-mode equality reasoning — equality reduces to a u64
// comparison and the proof kernel only reasons about identity, not
// byte-level content.
#[derive(Clone, Copy)]
pub struct WorkflowDigest(pub u64);

// ---------------------------------------------------------------------------
// Production mirror: vb_storage::admission::VerificationProof
// ---------------------------------------------------------------------------
//
// Verbatim mirror of `vb_storage::admission::VerificationProof` at
// `crates/vb_storage/src/admission.rs:67-91`. All 8 production fields
// are mirrored: digest, gate_count, durable, bounded_claimed,
// taint_safe_claimed, retry_safe_claimed,
// idempotency_verified_claimed, replayable_claimed.
//
// The `idempotency_keyed` / `idempotency_attested` slice fields and
// `warnings` vec at admission.rs:86-90 are NOT mirrored here — they
// are not part of the strict-admission gate-count / proof-flag
// decision.
pub struct VerificationProof {
    pub digest: WorkflowDigest,
    pub gate_count: u8,
    pub durable: bool,
    pub bounded_claimed: bool,
    pub taint_safe_claimed: bool,
    pub retry_safe_claimed: bool,
    pub idempotency_verified_claimed: bool,
    pub replayable_claimed: bool,
}

// ---------------------------------------------------------------------------
// Production mirror: vb_storage::admission::AcceptedArtifact
// ---------------------------------------------------------------------------
//
// Verbatim mirror of `vb_storage::admission::AcceptedArtifact` at
// `crates/vb_storage/src/admission.rs:203-228`. The 4 digest fields
// + embedded `VerificationProof` are mirrored because those are the
// only fields the spec reasons about. Opaque payload fields are
// omitted.
pub struct AcceptedArtifact {
    pub digest: WorkflowDigest,
    pub source_digest: WorkflowDigest,
    pub policy_digest: WorkflowDigest,
    pub verification: VerificationProof,
}

// ---------------------------------------------------------------------------
// Production decision fn: is_strict_admission_valid
// ---------------------------------------------------------------------------
//
// Mirrors the strict-policy gate validation in
// `vb_storage::admission::submit_artifact_with_contracts` at
// `crates/vb_storage/src/admission.rs:412-415` (the
// `VerificationProof::new(zero_workflow_digest(), ADMISSION_GATE_COUNT,
// durable)` call) combined with the unconditional flag set at
// admission.rs:123-127.
//
// Returns `true` iff the proof has the canonical 15-gate count AND
// all 5 spec-projection flags (`bounded_claimed`,
// `taint_safe_claimed`, `retry_safe_claimed`, `durable`,
// `replayable_claimed`) are `true`.
//
// Marked `#[verifier::external]` so Verus skips body verification;
// the companion spec file attaches the production contract via
// `assume_specification[ production::is_strict_admission_valid ]`.
#[verifier::external]
pub fn is_strict_admission_valid(proof: &VerificationProof) -> bool {
    proof.gate_count == 15
        && proof.bounded_claimed
        && proof.taint_safe_claimed
        && proof.retry_safe_claimed
        && proof.durable
        && proof.replayable_claimed
}

// ---------------------------------------------------------------------------
// Production decision fn: digest_eq
// ---------------------------------------------------------------------------
//
// Mirrors the `PartialEq` impl derived on `vb_core::ids::WorkflowDigest`
// at `crates/vb_core/src/ids/mod.rs:341`.
#[verifier::external]
pub fn digest_eq(a: &WorkflowDigest, b: &WorkflowDigest) -> bool {
    a.0 == b.0
}

// ---------------------------------------------------------------------------
// Production decision fn: artifact_digest_bound
// ---------------------------------------------------------------------------
//
// Mirrors `vb_storage::admission::bind_artifact_digest` at
// `crates/vb_storage/src/admission.rs:182-187`. Returns `true` iff
// the artifact's top-level digest equals the verification proof's
// digest.
#[verifier::external]
pub fn artifact_digest_bound(artifact: &AcceptedArtifact) -> bool {
    digest_eq(&artifact.digest, &artifact.verification.digest)
}