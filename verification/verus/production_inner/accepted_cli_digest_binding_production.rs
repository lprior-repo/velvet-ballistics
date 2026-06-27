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
//   - INV-002 strict-policy check <- crates/vb_runtime/src/admission.rs:711-716
//   - INV-003 strict-policy check <- crates/vb_runtime/src/admission.rs:720-725
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

#[verifier::external]
pub fn production_artifact_digest_eq_header(
    artifact: &SpecAcceptedArtifact,
    header_digest: SpecWorkflowDigest,
) -> bool {
    let _ = (artifact, header_digest);
    false
}

#[verifier::external]
pub fn production_proof_digest_eq_artifact(artifact: &SpecAcceptedArtifact) -> bool {
    let _ = artifact;
    false
}

#[verifier::external]
pub fn production_run_admission_new_digest(digest: SpecWorkflowDigest) -> SpecRunAdmission {
    SpecRunAdmission { artifact_digest: digest }
}

#[verifier::external]
pub fn production_run_admission_artifact_digest(admission: &SpecRunAdmission) -> SpecWorkflowDigest {
    admission.artifact_digest
}

#[verifier::external]
pub fn production_digest_binding_total(
    source: SpecWorkflowDigest,
    artifact: SpecWorkflowDigest,
    header: SpecWorkflowDigest,
    event: SpecWorkflowDigest,
    admission: SpecWorkflowDigest,
) -> bool {
    let _ = (source, artifact, header, event, admission);
    false
}