// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for accepted-run atomic admission
// ============================================================================
//
// This file mirrors the production strict-admission payload-tag
// surface and the prepared-proof gate-count decision.
//
// Production decision sources and unproved call-chain context:
//   - `vb_runtime::admission::REQUIRED_GATE_COUNT` (= 15)
//     (crates/vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs:2)
//   - `vb_storage::admission::submit_artifact_with_contracts`
//     (crates/vb_storage/src/admission.rs:96-105, unproved submit context)
//   - `vb_storage::admission::submit_prepared_artifact`
//     (crates/vb_storage/src/admission.rs:114-136, unproved Strict check/issuance/readback context)
//   - `vb_storage::admission::PayloadTag`, `classify_accepted_artifact`,
//     `is_strict_accepted_artifact_tag`
//     (crates/vb_storage/src/admission/payload_tag.rs:10-39)
//
// The 15-gate shape is NOT what production constructs or validates:
// `submit_artifact_with_contracts` performs no gate/flag validation.
// The prepared proof is built by `verification_proof_core`
// (crates/vb_storage/src/admission/proof.rs:103-118), which forces
// gate_count 0, durable false, and all flags false, and by
// `prepare_with_profile`
// (crates/vb_storage/src/admission/verification.rs:30-56), which sets
// only bounded_claimed and idempotency_verified_claimed true.
// `verify_accepted_artifact`
// (crates/vb_storage/src/admission/verification.rs:60-90) enforces the
// prepared shape by recompute-and-compare; any deviation is returned
// to callers as ArtifactEvidenceInvalid
// (crates/vb_runtime/src/admission/registered_verification.rs:5-28).
// `REQUIRED_GATE_COUNT` is used in production only by the
// AlwaysPresentArtifactStore fixture
// (crates/vb_runtime/src/admission/parts/chunk_003_stores.rs:74-91),
// which builds the legacy 15-gate/all-true shape with Unverified
// evidence; real validation rejects it.
//
// DRIFT POLICY: `crates/vb_storage/src/admission/payload_tag.rs:10-39`
// Decision coverage and unproved call-chain context:
//   - `REQUIRED_GATE_COUNT` (= 15)
//     <- crates/vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs:2
//   - `submit_artifact_with_contracts` (unproved context, not mirrored)
//     <- crates/vb_storage/src/admission.rs:96-105
//   - `submit_prepared_artifact` (unproved context, not mirrored)
//     <- crates/vb_storage/src/admission.rs:114-136
//   - `PayloadTag`, `classify_accepted_artifact`, `is_strict_accepted_artifact_tag`
//     <- crates/vb_storage/src/admission/payload_tag.rs:10-39
// Review this file when the cited definitions change. The payload-tag
// definitions have the separate Production claims below; the submit
// bodies are neither mirrored nor proved by this binding.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// `REQUIRED_GATE_COUNT` (= 15) at
// crates/vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs:2
// names the legacy fixture count; it is NOT the count production
// constructs. The prepared gate count is 0, forced by
// `verification_proof_core` at crates/vb_storage/src/admission/proof.rs:103-118.
// The literal `0` is inlined in the function body below to avoid
// Verus' `external_body` erasure issues with `pub const` items in
// included modules; the named const lives spec-side only.

use vstd::prelude::*;

verus! {

// Production `crates/vb_storage/src/admission/payload_tag.rs:10-23`
#[derive(Clone, Copy)]
pub enum PayloadTag {
    AcceptedArtifact,
    RawWorkflowParts,
    LegacyCompiledIr,
    Malformed,
}

// Local stubs for the production types `classify_accepted_artifact`
// reads: `SchemaVersion` and `CURRENT_IR_VERSION` from
// `vb_storage::compat`, `AcceptedArtifact` from `vb_storage::admission`.
#[derive(Clone, Copy)]
pub struct SchemaVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

pub const CURRENT_IR_VERSION: SchemaVersion = SchemaVersion { major: 2, minor: 0, patch: 0 };

pub struct AcceptedArtifact {
    pub artifact_version: SchemaVersion,
}

// Production `crates/vb_storage/src/admission/payload_tag.rs:27-33`
#[verifier::external]
pub fn classify_accepted_artifact(artifact: &AcceptedArtifact) -> PayloadTag {
    if artifact.artifact_version.major == CURRENT_IR_VERSION.major
        && artifact.artifact_version.minor == CURRENT_IR_VERSION.minor
        && artifact.artifact_version.patch == CURRENT_IR_VERSION.patch {
        PayloadTag::AcceptedArtifact
    } else {
        PayloadTag::LegacyCompiledIr
    }
}

// Production `crates/vb_storage/src/admission/payload_tag.rs:37-39`
#[verifier::external]
pub fn is_strict_accepted_artifact_tag(tag: PayloadTag) -> bool {
    matches!(tag, PayloadTag::AcceptedArtifact)
}

// Prepared-proof gate-count decision. Production constructs
// gate_count 0 (`verification_proof_core`,
// crates/vb_storage/src/admission/proof.rs:103-118); the bool is the
// prepared profile's claimed flags (bounded_claimed and
// idempotency_verified_claimed, set by `prepare_with_profile` at
// crates/vb_storage/src/admission/verification.rs:30-56). The legacy
// 15-gate shape exists only in the AlwaysPresentArtifactStore fixture
// (crates/vb_runtime/src/admission/parts/chunk_003_stores.rs:74-91)
// and is rejected by real validation.
#[verifier::external]
pub fn all_required_gates_accepted(
    gate_count: u8,
    all_required_gate_proofs_accepted: bool,
) -> bool {
    gate_count == 0 && all_required_gate_proofs_accepted
}

#[verifier::external]
pub fn artifact_matches_header_and_source(
    artifact_digest_matches: bool,
    workflow_digest_matches: bool,
    proof_matches: bool,
    capability_set_matches: bool,
) -> bool {
    artifact_digest_matches && workflow_digest_matches && proof_matches && capability_set_matches
}

#[verifier::external]
pub fn valid_commit_input(
    same_run: bool,
    same_workflow: bool,
    artifact_digest_matches: bool,
    workflow_digest_matches: bool,
    proof_matches: bool,
    capability_set_matches: bool,
    has_source: bool,
    has_artifact: bool,
    has_header: bool,
    has_runtime_policy: bool,
    has_capabilities: bool,
) -> bool {
    same_run && same_workflow && artifact_matches_header_and_source(
        artifact_digest_matches,
        workflow_digest_matches,
        proof_matches,
        capability_set_matches,
    ) && has_source && has_artifact && has_header && has_runtime_policy && has_capabilities
}

#[verifier::external]
pub fn bind_accepted_at_seq(
    artifact_run: i64,
    event_run: i64,
    accepted_at_seq: i64,
    run_accepted_seq: i64,
) -> bool {
    artifact_run == event_run && accepted_at_seq > 0 && accepted_at_seq == run_accepted_seq
}

#[verifier::external]
pub fn required_index_preconditions(
    committed: bool,
    status_points_to_run: bool,
    workflow_points_to_run: bool,
    action_points_to_run: bool,
) -> bool {
    committed && status_points_to_run && workflow_points_to_run && action_points_to_run
}

} // verus!
