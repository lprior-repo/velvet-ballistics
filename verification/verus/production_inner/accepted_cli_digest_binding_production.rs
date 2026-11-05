// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for accepted-cli digest binding
// ============================================================================
//
// This file is a VERBATIM mirror of the production runtime admission
// digest-binding types and decision fns.
//
// DRIFT POLICY: `crates/vb_runtime/src/admission.rs:82-725`
// Production source coverage:
//   - `WorkflowDigest`           <- crates/vb_core/src/ids/mod.rs:340-357
//   - `AcceptedArtifact`         <- crates/vb_storage/src/admission.rs:203-228
//   - `RunAdmission`             <- crates/vb_runtime/src/admission.rs:82-95
//   - `RunAdmission::new`        <- crates/vb_runtime/src/admission.rs:110-124
//   - `RunAdmission::artifact_digest`
//                                   <- crates/vb_runtime/src/admission.rs:162-166
//   - production_artifact_digest_eq_header
//                                   <- crates/vb_runtime/src/admission.rs:711-716
//                                     (INV-002 strict-policy check:
//                                      artifact.digest == header
//                                      || artifact.source_digest == header)
//   - production_proof_digest_eq_artifact
//                                   <- crates/vb_runtime/src/admission.rs:720-725
//                                     (INV-003 strict-policy check:
//                                      artifact.verification.digest == artifact.digest)
//   - production_digest_binding_total
//                                   <- crates/vb_runtime/src/admission.rs:711-725,
//                                      768-775
//                                     (5-digest chain post-condition:
//                                      source == artifact == header
//                                      == event == admission)
// Regenerate this file whenever production changes. Any rename of
// `digest`, `source_digest`, `verification.digest`, or `artifact_digest`
// breaks the `extern_accepted_cli_digest_binding` Verus build.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

#[derive(Clone, Copy)]
pub struct SpecWorkflowDigest(pub u64);

#[derive(Clone, Copy)]
pub enum SpecRuntimePolicy {
    Strict,
    Journaled,
    Relaxed,
    Other,
}

#[derive(Clone, Copy)]
pub struct SpecVerificationProof {
    pub digest: SpecWorkflowDigest,
}

#[derive(Clone, Copy)]
pub struct SpecAcceptedArtifact {
    pub digest: SpecWorkflowDigest,
    pub source_digest: SpecWorkflowDigest,
    pub policy_digest: SpecWorkflowDigest,
    pub verification: SpecVerificationProof,
}

#[derive(Clone, Copy)]
pub struct SpecRunAdmission {
    pub artifact_digest: SpecWorkflowDigest,
}

/// Mirror of the strict-policy INV-002 check at
/// `crates/vb_runtime/src/admission.rs:711-716`:
///
/// ```ignore
/// if artifact.digest != artifact_digest && artifact.source_digest != artifact_digest {
///     return Err(AdmissionError::ArtifactDigestMismatch { ... });
/// }
/// ```
///
/// Returns `true` iff the strict-policy check PASSES (no error), i.e.
/// `artifact.digest == header_digest || artifact.source_digest == header_digest`.
///
/// Equality is performed on the inner `u64` payload (the `SpecWorkflowDigest`
/// mirror intentionally omits `PartialEq` per the standard vstd
/// `discriminant_value` avoidance pattern; see
/// `verification/verus/production_inner/admission_artifact_production.rs:130`
/// for the same field-level equality pattern).
#[verifier::external]
pub fn production_artifact_digest_eq_header(
    artifact: &SpecAcceptedArtifact,
    header_digest: SpecWorkflowDigest,
) -> bool {
    artifact.digest.0 == header_digest.0 || artifact.source_digest.0 == header_digest.0
}

/// Mirror of the strict-policy INV-003 check at
/// `crates/vb_runtime/src/admission.rs:720-725`:
///
/// ```ignore
/// if artifact.verification.digest != artifact.digest {
///     return Err(AdmissionError::ArtifactDigestMismatch { ... });
/// }
/// ```
///
/// Returns `true` iff the strict-policy check PASSES (no error), i.e.
/// `artifact.verification.digest == artifact.digest`.
#[verifier::external]
pub fn production_proof_digest_eq_artifact(artifact: &SpecAcceptedArtifact) -> bool {
    artifact.verification.digest.0 == artifact.digest.0
}

#[verifier::external]
pub fn production_run_admission_new_digest(digest: SpecWorkflowDigest) -> SpecRunAdmission {
    SpecRunAdmission { artifact_digest: digest }
}

#[verifier::external]
pub fn production_run_admission_artifact_digest(admission: &SpecRunAdmission) -> SpecWorkflowDigest {
    admission.artifact_digest
}

/// Mirror of the 5-digest chain post-condition enforced by the
/// production strict-admission happy path
/// (`crates/vb_runtime/src/admission.rs:711-725` for INV-002 and INV-003,
/// and the `RunAdmission::with_idempotency_evidence` construction at
/// `admission.rs:768-775` which sets `admission.artifact_digest` to
/// `artifact.digest`).
///
/// Returns `true` iff every digest in the 5-element chain
/// `{source, artifact, header, event, admission}` is equal to the
/// single canonical envelope digest — the mathematical statement of
/// the production post-condition that all 5 digest positions
/// resolve to the same canonical envelope digest after successful
/// strict admission.
#[verifier::external]
pub fn production_digest_binding_total(
    source: SpecWorkflowDigest,
    artifact: SpecWorkflowDigest,
    header: SpecWorkflowDigest,
    event: SpecWorkflowDigest,
    admission: SpecWorkflowDigest,
) -> bool {
    source.0 == artifact.0
        && artifact.0 == header.0
        && header.0 == event.0
        && event.0 == admission.0
}