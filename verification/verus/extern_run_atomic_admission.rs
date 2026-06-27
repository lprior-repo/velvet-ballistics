// SPDX-License-Identifier: MIT
//
// Extern surface for accepted_run_atomic_admission Verus spec.
// Imports the production RunAdmission construction and decision logic:
//   - vb_runtime::admission::RunAdmission (struct)
//     at crates/vb_runtime/src/admission.rs:82-95
//   - vb_runtime::admission::RunAdmission::new
//     at crates/vb_runtime/src/admission.rs:110-124
//   - vb_runtime::admission::RunAdmission::with_idempotency_evidence
//     at crates/vb_runtime/src/admission.rs:127-142
//   - vb_runtime::admission::RunAdmission::with_budget
//     at crates/vb_runtime/src/admission.rs:145-160
//   - vb_runtime::admission::REQUIRED_GATE_COUNT (= 15)
//     at crates/vb_runtime/src/admission.rs:20

#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

// Canonical production constant. Mirrors
// crates/vb_runtime/src/admission.rs:20.
pub const REQUIRED_GATE_COUNT: u8 = 15;

/// Pure spec fn: payload tag for the strict admission policy. Mirrors the
/// accepted-artifact payload tag in
/// `vb_storage::admission::AcceptedArtifact` and the strict-policy branch
/// of `submit_artifact_with_contracts`.
pub enum PayloadTag {
    AcceptedArtifact,
    RawWorkflowParts,
    LegacyCompiledIr,
    Malformed,
}

/// Pure decision fn: is the payload tag a strict accepted artifact?
/// Mirrors the strict admission tag check at
/// `vb_storage::admission::submit_artifact_with_contracts` strict branch.
pub fn is_strict_accepted_artifact_tag(tag: PayloadTag) -> bool {
    matches!(tag, PayloadTag::AcceptedArtifact)
}

/// Pure decision fn: all required gates satisfied.
pub fn all_required_gates_accepted(gate_count: u8, all_required_gate_proofs_accepted: bool) -> bool {
    gate_count == REQUIRED_GATE_COUNT && all_required_gate_proofs_accepted
}

/// Pure decision fn: artifact matches header and source digests.
pub fn artifact_matches_header_and_source(
    artifact_digest_matches: bool,
    workflow_digest_matches: bool,
    proof_matches: bool,
    capability_set_matches: bool,
) -> bool {
    artifact_digest_matches && workflow_digest_matches && proof_matches && capability_set_matches
}

/// Pure decision fn: valid commit input.
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

/// Pure decision fn: bind accepted_at_seq to RunAccepted.seq.
pub fn bind_accepted_at_seq(artifact_run: i64, event_run: i64, accepted_at_seq: i64, run_accepted_seq: i64) -> bool {
    artifact_run == event_run && accepted_at_seq > 0 && accepted_at_seq == run_accepted_seq
}

/// Pure decision fn: required index preconditions.
pub fn required_index_preconditions(
    committed: bool,
    status_points_to_run: bool,
    workflow_points_to_run: bool,
    action_points_to_run: bool,
) -> bool {
    committed && status_points_to_run && workflow_points_to_run && action_points_to_run
}
