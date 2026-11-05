// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for capability artifact model
// ============================================================================
//
// This file is a VERBATIM mirror of the production capability admission
// types and decision logic.
//
// Production sources mirrored:
//   - `vb_core::capability::Capability`     (crates/vb_core/src/capability.rs:10-27)
//   - `vb_core::policy::RuntimePolicy`       (crates/vb_core/src/policy.rs)
//   - `vb_runtime::admission::AdmissionError` (crates/vb_runtime/src/admission.rs:200-331)
//   - `vb_runtime::admission::admit_artifact_run_with_certificate_floor`
//                                              (crates/vb_runtime/src/admission.rs:692-785)
//
// Stub substitutions:
//   - `Capability` projected to `SpecCapability { name_hash: u64, action: u16 }`
//   - `RuntimePolicy` projected to `SpecRuntimePolicy`
//   - `AdmissionError` projected to `SpecAdmitError`
//   - `admit_artifact_run_with_certificate_floor` projected to a pure decision fn.
//
// DRIFT POLICY: `crates/vb_runtime/src/admission.rs:200-785`
// Production source coverage:
//   - `Capability`            <- crates/vb_core/src/capability.rs:10-27
//   - `RuntimePolicy`         <- crates/vb_core/src/policy.rs
//   - `AdmissionError`        <- crates/vb_runtime/src/admission.rs:200-331
//   - `admit_artifact_run_with_certificate_floor`
//                                  <- crates/vb_runtime/src/admission.rs:692-785
// Regenerate this file whenever production changes. Any rename of
// `Capability`, `RuntimePolicy`, or `AdmissionError` variants breaks
// the `extern_capability_artifact_model` Verus build.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// `SpecCapability` mirror of `vb_core::capability::Capability`.
#[derive(Clone, Copy)]
pub struct SpecCapability {
    pub name: u64,
    pub action: u16,
}

#[derive(Clone, Copy)]
pub enum SpecRuntimePolicy {
    Strict,
    Journaled,
    Relaxed,
    Other,
}

#[derive(Clone, Copy)]
pub enum SpecAdmitError {
    Ok,
    CapabilityCountMismatch { required_count: u64, granted_count: u64 },
    CapabilityDenied,
    ArtifactNotFound,
    ArtifactEnvelopeDecodeFailed,
    ArtifactInvalidGateCount,
    ArtifactInvalidProofFlag,
    ArtifactDigestMismatch,
    ArtifactCertificateStale,
}

#[verifier::external]
pub fn admit_artifact_run_with_certificate_floor(
    policy: SpecRuntimePolicy,
    required_count: u64,
    granted_count: u64,
    all_required_granted: bool,
    earlier_gates_passed: bool,
) -> SpecAdmitError {
    match policy {
        SpecRuntimePolicy::Strict | SpecRuntimePolicy::Journaled => {
            if !earlier_gates_passed {
                SpecAdmitError::ArtifactDigestMismatch
            } else if !all_required_granted {
                SpecAdmitError::CapabilityDenied
            } else if required_count != granted_count {
                SpecAdmitError::CapabilityCountMismatch { required_count, granted_count }
            } else {
                SpecAdmitError::Ok
            }
        }
        SpecRuntimePolicy::Relaxed => SpecAdmitError::Ok,
        SpecRuntimePolicy::Other => SpecAdmitError::ArtifactInvalidProofFlag,
    }
}