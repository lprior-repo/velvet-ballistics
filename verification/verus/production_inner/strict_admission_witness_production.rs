// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for strict-admission witness
// ============================================================================
//
// This file is a VERBATIM mirror of the production strict-admission
// witness types and decision fns.
//
// DRIFT POLICY: `crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:83-189`
// Production source coverage:
//   - `SpecRuntimePolicy`  <- mirror of `vb_core::policy::RuntimePolicy`
//                                (crates/vb_core/src/policy.rs)
//   - `SpecWitnessKind`    <- mirror of strict-admission witness kinds
//   - `SpecStrictWitnessResult`
//                              <- mirror of strict-admission decision
//                                 output variants at
//                                 crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:83-189
//   - `production_strict_like`        <- production strict-policy gate
//   - `production_storage_backed`     <- production storage-backed gate
//   - `strict_admission_witness_decision`
//                                       <- production composite decision fn:
//                                          strict-like policy plus a
//                                          storage-backed witness plus the
//                                          prepared-proof shape
//                                          (verification_proof_core at
//                                          crates/vb_storage/src/admission.rs:113-128;
//                                          enforced by verify_accepted_artifact at
//                                          crates/vb_storage/src/admission/verification.rs:56-86,
//                                          which rejects any deviation with
//                                          ArtifactEvidenceInvalid). The
//                                          legacy 15-gate shape is the
//                                          AlwaysPresentArtifactStore fixture
//                                          only
//                                          (crates/vb_runtime/src/admission/parts/chunk_003_stores.rs:74-91).
// Regenerate this file whenever production changes. Any rename of
// `SpecRuntimePolicy`, `SpecWitnessKind`, or `SpecStrictWitnessResult`
// variants breaks the `extern_strict_admission_witness` Verus build
// at compile time.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

#[derive(Clone, Copy)]
pub enum SpecRuntimePolicy {
    Strict,
    Journaled,
    Relaxed,
    Other,
}

#[derive(Clone, Copy)]
pub enum SpecWitnessKind {
    StorageAcceptedArtifact,
    RawWorkflowParts,
    RawCompiledWorkflow,
    AlwaysPresentStore,
}

#[derive(Clone, Copy)]
pub enum SpecStrictWitnessResult {
    StrictAccepted,
    NotStrictLike,
    WitnessNotStorageBacked,
    GateCountInvalid,
    RequiredProofFlagMissing,
}

#[verifier::external]
pub fn production_strict_like(policy: SpecRuntimePolicy) -> bool {
    match policy {
        SpecRuntimePolicy::Strict => true,
        SpecRuntimePolicy::Journaled => true,
        SpecRuntimePolicy::Relaxed => false,
        SpecRuntimePolicy::Other => false,
    }
}

#[verifier::external]
pub fn production_storage_backed(witness: SpecWitnessKind) -> bool {
    match witness {
        SpecWitnessKind::StorageAcceptedArtifact => true,
        SpecWitnessKind::RawWorkflowParts => false,
        SpecWitnessKind::RawCompiledWorkflow => false,
        SpecWitnessKind::AlwaysPresentStore => false,
    }
}

#[verifier::external]
pub fn strict_admission_witness_decision(
    policy: SpecRuntimePolicy,
    witness: SpecWitnessKind,
    gate_count: u8,
    prepared_proof_shape: bool,
) -> SpecStrictWitnessResult {
    if !production_strict_like(policy) {
        SpecStrictWitnessResult::NotStrictLike
    } else if !production_storage_backed(witness) {
        SpecStrictWitnessResult::WitnessNotStorageBacked
    } else if gate_count != 0 {
        // Prepared gate count: `verification_proof_core` forces
        // gate_count 0 for every admitted artifact
        // (crates/vb_storage/src/admission.rs:113-128). The literal is
        // used here rather than a named const: a `pub const` in this
        // mirror triggers a known Verus erasure bug (see the
        // spec-side PREPARED_GATE_COUNT in strict_admission_witness.rs
        // for the same work-around as REQUIRED_GATE_COUNT).
        SpecStrictWitnessResult::GateCountInvalid
    } else if !prepared_proof_shape {
        SpecStrictWitnessResult::RequiredProofFlagMissing
    } else {
        SpecStrictWitnessResult::StrictAccepted
    }
}
