// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for accepted-cli digest binding
// ============================================================================
//
// This file is a handwritten structural mirror of selected production
// admission fields and isolated digest predicates, not a verbatim copy.
//
// DRIFT POLICY: `crates/vb_runtime/src/admission/parts/chunk_002_records.rs:6-34`
//                + `crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:91-130,156-173,208-224`
// Production source coverage:
//   - `WorkflowDigest`           <- crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:241-255
//   - `AcceptedArtifact`         <- crates/vb_storage/src/admission.rs:41-77
//   - `RunAdmission`             <- crates/vb_runtime/src/admission/parts/chunk_002_records.rs:6-34
//   - `RunAdmission::new`        <- crates/vb_runtime/src/admission/parts/chunk_002_records.rs:51-69
//   - `RunAdmission::artifact_digest`
//                                   <- crates/vb_runtime/src/admission/parts/chunk_002_records.rs:161-165
//   - digest acceptance and proof-digest checks
//                                   <- `verify_admission_digest` at
//                                      crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:156-173
//   - returned RunAdmission construction
//                                   <- `admission_from_artifact` at
//                                      crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:208-224
// The production check permits a request matching `source_digest` while
// `artifact.digest` differs. Thus the mirror's 5-way equality predicate
// is stronger than this production check; the mirror does not prove the
// entire admission call chain.
// Regenerate this file when the mirrored production fields or decision
// conditions change. Names and signatures in the mirror detect only
// the structural drift actually represented here.
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

/// Mirror of the artifact/request condition in `verify_admission_digest`
/// at `crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:156-165`:
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
/// `verification/verus/production_inner/admission_artifact_production.rs:406-408`
/// for the same field-level equality pattern).
#[verifier::external]
pub fn production_artifact_digest_eq_header(
    artifact: &SpecAcceptedArtifact,
    header_digest: SpecWorkflowDigest,
) -> bool {
    artifact.digest.0 == header_digest.0 || artifact.source_digest.0 == header_digest.0
}

/// Mirror of the verification-digest condition in `verify_admission_digest`
/// at `crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:166-173`:
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
    SpecRunAdmission {
        artifact_digest: digest,
    }
}

#[verifier::external]
pub fn production_run_admission_artifact_digest(
    admission: &SpecRunAdmission,
) -> SpecWorkflowDigest {
    admission.artifact_digest
}

/// This mirror expresses a standalone 5-way equality predicate. Production's
/// `verify_admission_digest` at
/// `crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:156-173`
/// allows the request to match `source_digest` while `artifact.digest`
/// differs, so successful production admission does not establish this
/// predicate for all inputs. This mirror does not prove the complete
/// admission call chain.
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
