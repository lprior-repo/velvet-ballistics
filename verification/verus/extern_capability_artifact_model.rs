// SPDX-License-Identifier: MIT
//
// Extern surface for capability_artifact_model Verus spec.
//
// Production binding (BINDING LEDGER):
//   - Capability at crates/vb_core/src/capability.rs:10-27
//     { name: Box<str>, action: ActionId(u16) }
//     Projection: SpecCapability { name: int, action: nat }
//   - CapabilitySet at crates/vb_core/src/capability.rs:31-78
//     Projection: SpecCapability's all_required_granted flag
//     (the membership check is performed in spec mode via the
//     all_required_granted quantifier in the companion spec file).
//   - RuntimePolicy at crates/vb_core/src/policy.rs
//     (Strict | Journaled | Relaxed | …)
//     Projection: SpecRuntimePolicy.
//   - admit_artifact_run_with_certificate_floor at
//     crates/vb_runtime/src/admission.rs:692-785
//     Projection: admit_artifact_run_with_certificate_floor below,
//     capturing the cardinality-exact branch (admission.rs:735-766)
//     and the policy dispatch (admission.rs:700-784).
//   - AdmissionError at crates/vb_runtime/src/admission.rs:200-?
//     15 variants total. Projection: SpecAdmitError (9 variants
//     covering the strict-policy cardinality-exact branch plus a
//     general failure surface for unmodeled branches).
//
// TRUST BOUNDARY: the production body of
// `admit_artifact_run_with_certificate_floor` is not verified here.
// The body below is `#[verifier::external]`; the spec file attaches
// `assume_specification` to it and exercises the contract via the
// `checked_admit_artifact_run_with_certificate_floor` exec wrapper.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

// ---------------------------------------------------------------------------
// Mirror types
// ---------------------------------------------------------------------------

/// Mirror of vb_core::capability::Capability { name: Box<str>, action: ActionId(u16) }.
///
/// `name: int` is an abstract name identity (production `Box<str>` projected
/// to `int` via a stable hash, equality-preserving for exact-match checks).
/// `action: nat` models `ActionId` (newtype around `u16`, total over
/// 0..=u16::MAX).
#[derive(Clone, Copy)]
pub struct SpecCapability {
    pub name: int,
    pub action: nat,
}

/// Mirror of vb_core::policy::RuntimePolicy. Only the three variants
/// the production `admit_artifact_run_with_certificate_floor` actually
/// dispatches on are enumerated; anything else collapses to `Other`
/// which maps to `AdmissionError::ArtifactInvalidProofFlag { flag:
/// "runtime_policy" }` in production.
#[derive(Clone, Copy)]
pub enum SpecRuntimePolicy {
    Strict,
    Journaled,
    Relaxed,
    Other,
}

/// Mirror of vb_runtime::admission::AdmissionError subset (production
/// 15 variants; this projection enumerates the 9 variants the
/// cardinality-exact proof is contracted to reason about plus a
/// general failure surface for unmodeled branches).
///
/// Mapping (production -> projection):
///   Ok                                         -> Ok
///   CapabilityCountMismatch { required, grant } -> CapabilityCountMismatch
///   CapabilityDenied { .. }                     -> CapabilityDenied
///   ArtifactNotFound { .. }                     -> ArtifactNotFound
///   ArtifactEnvelopeDecodeFailed                -> ArtifactEnvelopeDecodeFailed
///   ArtifactInvalidGateCount { .. }             -> ArtifactInvalidGateCount
///   ArtifactInvalidProofFlag { .. }             -> ArtifactInvalidProofFlag
///   ArtifactDigestMismatch { .. }               -> ArtifactDigestMismatch
///   ArtifactCertificateStale { .. }             -> ArtifactCertificateStale
///
/// The remaining 6 production variants (ResourceCapacityExceeded,
/// BudgetPolicyExceeded, ResourceBudgetOverflow, ResourceBudgetUnderflow,
/// ResourceBudgetInvalidCapacity, ResourceStepCeilingExceeded,
/// ResourcePerTickCeilingExceeded) are budget-arithmetic errors and are
/// out of scope for the capability-cardinality proof; the rest of the
/// surface is collapsed into the unmatched `ArtifactInvalidProofFlag`
/// arm in `spec_admit_decision`.
#[derive(Clone, Copy)]
pub enum SpecAdmitError {
    Ok,
    CapabilityCountMismatch {
        required_count: u64,
        granted_count: u64,
    },
    CapabilityDenied,
    ArtifactNotFound,
    ArtifactEnvelopeDecodeFailed,
    ArtifactInvalidGateCount,
    ArtifactInvalidProofFlag,
    ArtifactDigestMismatch,
    ArtifactCertificateStale,
}

// ---------------------------------------------------------------------------
// Spec predicates (math layer) — const-fn mirrors for documentation
// ---------------------------------------------------------------------------

/// Mirror of `spec_exact_capability_match` (the spec fn lives
/// in the spec file). Returns true iff two `SpecCapability` values
/// match exactly (name identity and action discriminant).
///
/// Note: declared `pub fn` rather than `pub const fn` because
/// `int` and `nat` equality is not a const operator in Verus; the
/// spec fn mirror in the companion file is what proofs reference.
pub fn spec_exact_capability_match(required: SpecCapability, granted: SpecCapability) -> bool {
    required.name == granted.name && required.action == granted.action
}

// ---------------------------------------------------------------------------
// Pure projection: admit_artifact_run_with_certificate_floor decision
// ---------------------------------------------------------------------------

/// Pure projection of the cardinality-exact branch of
/// `vb_runtime::admission::admit_artifact_run_with_certificate_floor`
/// at crates/vb_runtime/src/admission.rs:692-785.
///
/// Inputs:
///   - `policy`: which RuntimePolicy arm the production dispatch enters
///     (production: admission.rs:700).
///   - `required_count`: the artifact's required capability count
///     (projection of `artifact.required_capabilities.len()`).
///   - `granted_count`: the granted capability count (projection of
///     `caps.len()`).
///   - `all_required_granted`: bool flag encoding whether every required
///     capability has an exact name+action match in the granted set
///     (production membership check: admission.rs:756-758).
///   - `earlier_gates_passed`: bool flag encoding whether the digest
///     binding + certificate staleness checks at admission.rs:711-733
///     passed. False collapses to a generic `ArtifactDigestMismatch`
///     for unmodeled upstream failures.
///
/// Output: `SpecAdmitError` mirroring the production `AdmissionError`
/// variant for the cardinality-exact branch (admission.rs:740-766):
///   - Strict/Journaled + earlier_gates_passed + all_required_granted +
///     required_count == granted_count -> Ok
///   - Strict/Journaled + earlier_gates_passed + !all_required_granted ->
///     CapabilityDenied (production: from `check_capability` at
///     admission.rs:756-758)
///   - Strict/Journaled + earlier_gates_passed + all_required_granted +
///     required_count != granted_count -> CapabilityCountMismatch { .. }
///     (production: admission.rs:761-766)
///   - Strict/Journaled + !earlier_gates_passed -> ArtifactDigestMismatch
///     (production: admission.rs:711-733, collapsed for the projection)
///   - Relaxed -> Ok (production: admission.rs:777-780)
///   - Other -> ArtifactInvalidProofFlag (production: admission.rs:781-783)
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The companion spec file attaches `assume_specification` to this fn
/// and the contract discharges the cardinality-exact obligation.
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
                SpecAdmitError::CapabilityCountMismatch {
                    required_count,
                    granted_count,
                }
            } else {
                SpecAdmitError::Ok
            }
        }
        SpecRuntimePolicy::Relaxed => SpecAdmitError::Ok,
        SpecRuntimePolicy::Other => SpecAdmitError::ArtifactInvalidProofFlag,
    }
}
