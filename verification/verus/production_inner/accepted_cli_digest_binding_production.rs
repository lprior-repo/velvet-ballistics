// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for accepted-cli digest binding
// ============================================================================
//
// This file is a VERBATIM mirror of the production runtime admission
// digest-binding types and decision fns.

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