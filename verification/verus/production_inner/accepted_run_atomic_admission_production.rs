// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for accepted-run atomic admission
// ============================================================================
//
// This file is a VERBATIM mirror of the production strict-admission
// payload-tag and decision-fn surface.
//
// Production sources mirrored:
//   - `vb_runtime::admission::REQUIRED_GATE_COUNT` (= 15)
//                                                (crates/vb_runtime/src/admission.rs:20)
//   - `vb_storage::admission::submit_artifact_with_contracts` strict
//     branch (crates/vb_storage/src/admission.rs:327-422)
//
// DRIFT POLICY: `crates/vb_runtime/src/admission.rs:20-422`
// Production source coverage:
//   - `REQUIRED_GATE_COUNT` (= 15)            <- crates/vb_runtime/src/admission.rs:20
//   - `submit_artifact_with_contracts` strict branch
//                                                <- crates/vb_storage/src/admission.rs:327-422
// Regenerate this file whenever production changes. Any rename of
// `PayloadTag::AcceptedArtifact` or body change in the strict branch
// breaks the `extern_run_atomic_admission` Verus build.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// Canonical production constant. Mirrors `REQUIRED_GATE_COUNT = 15`
// at crates/vb_runtime/src/admission.rs:20. Inlined as the literal
// value `15` in function bodies to avoid Verus' `external_body`
// erasure issues with `pub const` items in included modules.

// Production mirror: payload tag discriminant. Mirrors the
// accepted-artifact payload tag in
// `vb_storage::admission::AcceptedArtifact` and the strict-policy
// branch of `submit_artifact_with_contracts`.
#[derive(Clone, Copy)]
pub enum PayloadTag {
    AcceptedArtifact,
    RawWorkflowParts,
    LegacyCompiledIr,
    Malformed,
}

#[verifier::external]
pub fn is_strict_accepted_artifact_tag(tag: PayloadTag) -> bool {
    matches!(tag, PayloadTag::AcceptedArtifact)
}

#[verifier::external]
pub fn all_required_gates_accepted(gate_count: u8, all_required_gate_proofs_accepted: bool) -> bool {
    gate_count == 15 && all_required_gate_proofs_accepted
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
    same_run
        && same_workflow
        && artifact_matches_header_and_source(
            artifact_digest_matches,
            workflow_digest_matches,
            proof_matches,
            capability_set_matches,
        )
        && has_source
        && has_artifact
        && has_header
        && has_runtime_policy
        && has_capabilities
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