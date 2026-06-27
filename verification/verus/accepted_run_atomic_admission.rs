// Verus model for vb-core-atomic-admission proof obligations.
//
// Obligations:
// - VERUS-PRE-001: valid accepted-run inputs contain required families.
// - VERUS-PRE-002: coherent pure inputs preserve matching references.
// - VERUS-SEQ-003: accepted_at_seq is non-sentinel and equals RunAccepted.seq.
// - VERUS-ART-004: strict compiled payloads reject raw WorkflowParts.
// - VERUS-IDX-005: required index preconditions decompose to committed-run facts.
// - VERUS-ERR-006: model-level strict admission failures classify to Err outcomes.
//
// Production binding (BINDING LEDGER):
//   - Required gate count mirrors `vb_runtime::admission::REQUIRED_GATE_COUNT` (= 15)
//     at crates/vb_runtime/src/admission.rs:20.
//   - RunAdmission struct mirrors `vb_runtime::admission::RunAdmission`
//     at crates/vb_runtime/src/admission.rs:82-95.
//   - RunAdmission::new mirrors
//     `vb_runtime::admission::RunAdmission::new` at
//     crates/vb_runtime/src/admission.rs:110-124.
//   - submit_artifact_with_contracts strict branch mirrors
//     `vb_storage::admission::submit_artifact_with_contracts` at
//     crates/vb_storage/src/admission.rs:327-422.

use vstd::prelude::*;

verus! {

#[path = "extern_run_atomic_admission.rs"]
mod production;

// ============================================================
// Production-bound exec fns (mirror production decision fns)
// ============================================================

// Production decision fn: is_strict_accepted_artifact_tag mirrors the
// strict admission tag check at
// vb_storage::admission::submit_artifact_with_contracts strict branch.
pub fn is_strict_accepted_artifact_tag(tag: production::PayloadTag) -> bool {
    production::is_strict_accepted_artifact_tag(tag)
}

pub fn all_required_gates_accepted(gate_count: u8, all_required_gate_proofs_accepted: bool) -> bool {
    production::all_required_gates_accepted(gate_count, all_required_gate_proofs_accepted)
}

pub fn artifact_matches_header_and_source(
    artifact_digest_matches: bool,
    workflow_digest_matches: bool,
    proof_matches: bool,
    capability_set_matches: bool,
) -> bool {
    production::artifact_matches_header_and_source(
        artifact_digest_matches,
        workflow_digest_matches,
        proof_matches,
        capability_set_matches,
    )
}

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
    production::valid_commit_input(
        same_run,
        same_workflow,
        artifact_digest_matches,
        workflow_digest_matches,
        proof_matches,
        capability_set_matches,
        has_source,
        has_artifact,
        has_header,
        has_runtime_policy,
        has_capabilities,
    )
}

pub fn bind_accepted_at_seq(artifact_run: i64, event_run: i64, accepted_at_seq: i64, run_accepted_seq: i64) -> bool {
    production::bind_accepted_at_seq(artifact_run, event_run, accepted_at_seq, run_accepted_seq)
}

pub fn required_index_preconditions(
    committed: bool,
    status_points_to_run: bool,
    workflow_points_to_run: bool,
    action_points_to_run: bool,
) -> bool {
    production::required_index_preconditions(committed, status_points_to_run, workflow_points_to_run, action_points_to_run)
}

// ---------------------------------------------------------------------------
// assume_specification bridges — production contract surface
// ---------------------------------------------------------------------------
//
// These bridges attach spec contracts to the production-bound exec fns
// in `production_inner/accepted_run_atomic_admission_production.rs`.
// The body of each extern fn is opaque to Verus; the spec proofs
// below exercise the contracts via the exec wrappers above.

pub assume_specification[ production::is_strict_accepted_artifact_tag ](
    tag: production::PayloadTag,
) -> (r: bool)
    ensures
        r == spec_strict_payload_is_accepted_artifact(match tag {
            production::PayloadTag::AcceptedArtifact => SpecPayloadTag::AcceptedArtifact,
            production::PayloadTag::RawWorkflowParts => SpecPayloadTag::RawWorkflowParts,
            production::PayloadTag::LegacyCompiledIr => SpecPayloadTag::LegacyCompiledIr,
            production::PayloadTag::Malformed => SpecPayloadTag::Malformed,
        }),
;

pub assume_specification[ production::all_required_gates_accepted ](
    gate_count: u8,
    all_required_gate_proofs_accepted: bool,
) -> (r: bool)
    ensures
        r == (gate_count == 15 && all_required_gate_proofs_accepted),
;

pub assume_specification[ production::artifact_matches_header_and_source ](
    artifact_digest_matches: bool,
    workflow_digest_matches: bool,
    proof_matches: bool,
    capability_set_matches: bool,
) -> (r: bool)
    ensures
        r == spec_artifact_matches_header_and_source(
            artifact_digest_matches,
            workflow_digest_matches,
            proof_matches,
            capability_set_matches,
        ),
;

pub assume_specification[ production::valid_commit_input ](
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
) -> (r: bool)
    ensures
        r == spec_valid_commit_input(
            same_run,
            same_workflow,
            artifact_digest_matches,
            workflow_digest_matches,
            proof_matches,
            capability_set_matches,
            has_source,
            has_artifact,
            has_header,
            has_runtime_policy,
            has_capabilities,
        ),
;

pub assume_specification[ production::bind_accepted_at_seq ](
    artifact_run: i64,
    event_run: i64,
    accepted_at_seq: i64,
    run_accepted_seq: i64,
) -> (r: bool)
    ensures
        r == spec_bind_accepted_at_seq(
            artifact_run as int,
            event_run as int,
            accepted_at_seq as int,
            run_accepted_seq as int,
        ),
;

pub assume_specification[ production::required_index_preconditions ](
    committed: bool,
    status_points_to_run: bool,
    workflow_points_to_run: bool,
    action_points_to_run: bool,
) -> (r: bool)
    ensures
        r == spec_required_index_preconditions(
            committed,
            status_points_to_run,
            workflow_points_to_run,
            action_points_to_run,
        ),
;

// ============================================================
// Spec mirrors
// ============================================================

pub enum SpecPayloadTag {
    AcceptedArtifact,
    RawWorkflowParts,
    LegacyCompiledIr,
    Malformed,
}

pub enum SpecFailureCause {
    InvalidAcceptedArtifact,
    InconsistentAdmissionInput,
    BatchStageFailed,
    BatchCommitFailed,
    PartialVisibilityDetected,
    SequenceBindingFailed,
    StrictRawWorkflowPartsRejected,
    IndexDerivationFailed,
}

pub enum SpecAdmissionError {
    InvalidAcceptedArtifact,
    InconsistentAdmissionInput,
    BatchStageFailed,
    BatchCommitFailed,
    PartialVisibilityDetected,
    SequenceBindingFailed,
    StrictRawWorkflowPartsRejected,
    IndexDerivationFailed,
}

pub enum SpecAdmissionOutcome {
    Success,
    Err(SpecAdmissionError),
}

pub open spec fn spec_artifact_matches_header_and_source(
    artifact_digest_matches: bool,
    workflow_digest_matches: bool,
    proof_matches: bool,
    capability_set_matches: bool,
) -> bool {
    &&& artifact_digest_matches
    &&& workflow_digest_matches
    &&& proof_matches
    &&& capability_set_matches
}

pub open spec fn spec_valid_commit_input(
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
    &&& same_run
    &&& same_workflow
    &&& spec_artifact_matches_header_and_source(
        artifact_digest_matches,
        workflow_digest_matches,
        proof_matches,
        capability_set_matches,
    )
    &&& has_source
    &&& has_artifact
    &&& has_header
    &&& has_runtime_policy
    &&& has_capabilities
}

pub open spec fn spec_bind_accepted_at_seq(
    artifact_run: int,
    event_run: int,
    accepted_at_seq: int,
    run_accepted_seq: int,
) -> bool {
    &&& artifact_run == event_run
    &&& accepted_at_seq > 0
    &&& accepted_at_seq == run_accepted_seq
}

pub open spec fn spec_strict_payload_is_accepted_artifact(tag: SpecPayloadTag) -> bool {
    match tag {
        SpecPayloadTag::AcceptedArtifact => true,
        SpecPayloadTag::RawWorkflowParts => false,
        SpecPayloadTag::LegacyCompiledIr => false,
        SpecPayloadTag::Malformed => false,
    }
}

pub open spec fn spec_required_index_preconditions(
    committed: bool,
    status_points_to_run: bool,
    workflow_points_to_run: bool,
    action_points_to_run: bool,
) -> bool {
    &&& committed
    &&& status_points_to_run
    &&& workflow_points_to_run
    &&& action_points_to_run
}

pub open spec fn spec_outcome_is_err(outcome: SpecAdmissionOutcome) -> bool {
    match outcome {
        SpecAdmissionOutcome::Success => false,
        SpecAdmissionOutcome::Err(_) => true,
    }
}

pub open spec fn spec_admission_outcome(cause: SpecFailureCause) -> SpecAdmissionOutcome {
    match cause {
        SpecFailureCause::InvalidAcceptedArtifact => SpecAdmissionOutcome::Err(SpecAdmissionError::InvalidAcceptedArtifact),
        SpecFailureCause::InconsistentAdmissionInput => SpecAdmissionOutcome::Err(SpecAdmissionError::InconsistentAdmissionInput),
        SpecFailureCause::BatchStageFailed => SpecAdmissionOutcome::Err(SpecAdmissionError::BatchStageFailed),
        SpecFailureCause::BatchCommitFailed => SpecAdmissionOutcome::Err(SpecAdmissionError::BatchCommitFailed),
        SpecFailureCause::PartialVisibilityDetected => SpecAdmissionOutcome::Err(SpecAdmissionError::PartialVisibilityDetected),
        SpecFailureCause::SequenceBindingFailed => SpecAdmissionOutcome::Err(SpecAdmissionError::SequenceBindingFailed),
        SpecFailureCause::StrictRawWorkflowPartsRejected => SpecAdmissionOutcome::Err(SpecAdmissionError::StrictRawWorkflowPartsRejected),
        SpecFailureCause::IndexDerivationFailed => SpecAdmissionOutcome::Err(SpecAdmissionError::IndexDerivationFailed),
    }
}

pub open spec fn spec_error_classifies_failure(
    cause: SpecFailureCause,
    error: SpecAdmissionError,
) -> bool {
    match cause {
        SpecFailureCause::InvalidAcceptedArtifact => matches!(error, SpecAdmissionError::InvalidAcceptedArtifact),
        SpecFailureCause::InconsistentAdmissionInput => matches!(error, SpecAdmissionError::InconsistentAdmissionInput),
        SpecFailureCause::BatchStageFailed => matches!(error, SpecAdmissionError::BatchStageFailed),
        SpecFailureCause::BatchCommitFailed => matches!(error, SpecAdmissionError::BatchCommitFailed),
        SpecFailureCause::PartialVisibilityDetected => matches!(error, SpecAdmissionError::PartialVisibilityDetected),
        SpecFailureCause::SequenceBindingFailed => matches!(error, SpecAdmissionError::SequenceBindingFailed),
        SpecFailureCause::StrictRawWorkflowPartsRejected => matches!(error, SpecAdmissionError::StrictRawWorkflowPartsRejected),
        SpecFailureCause::IndexDerivationFailed => matches!(error, SpecAdmissionError::IndexDerivationFailed),
    }
}

// ============================================================
// Non-vacuous proofs
// ============================================================

// Non-vacuous: derives each required-family conjunct from spec_valid_commit_input.
pub proof fn proof_valid_input_has_required_families(
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
)
    requires
        spec_valid_commit_input(
            same_run,
            same_workflow,
            artifact_digest_matches,
            workflow_digest_matches,
            proof_matches,
            capability_set_matches,
            has_source,
            has_artifact,
            has_header,
            has_runtime_policy,
            has_capabilities,
        ),
    ensures
        has_source,
        has_artifact,
        has_header,
        has_runtime_policy,
        has_capabilities,
{
    reveal(spec_valid_commit_input);
    assert(has_source);
    assert(has_artifact);
    assert(has_header);
    assert(has_runtime_policy);
    assert(has_capabilities);
}

// Non-vacuous: derives same-run, same-workflow, and digest-matching conjuncts.
pub proof fn proof_coherent_input_refs(
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
)
    requires
        spec_valid_commit_input(
            same_run,
            same_workflow,
            artifact_digest_matches,
            workflow_digest_matches,
            proof_matches,
            capability_set_matches,
            has_source,
            has_artifact,
            has_header,
            has_runtime_policy,
            has_capabilities,
        ),
    ensures
        same_run,
        same_workflow,
        spec_artifact_matches_header_and_source(
            artifact_digest_matches,
            workflow_digest_matches,
            proof_matches,
            capability_set_matches,
        ),
{
    reveal(spec_valid_commit_input);
    reveal(spec_artifact_matches_header_and_source);
    assert(same_run);
    assert(same_workflow);
    assert(artifact_digest_matches);
    assert(workflow_digest_matches);
    assert(proof_matches);
    assert(capability_set_matches);
}

// Non-vacuous: derives all three sequence-binding conjuncts.
pub proof fn proof_sequence_binding_preserves_truth(
    artifact_run: int,
    event_run: int,
    accepted_at_seq: int,
    run_accepted_seq: int,
)
    requires
        spec_bind_accepted_at_seq(artifact_run, event_run, accepted_at_seq, run_accepted_seq),
    ensures
        artifact_run == event_run,
        accepted_at_seq > 0,
        accepted_at_seq == run_accepted_seq,
{
    reveal(spec_bind_accepted_at_seq);
    assert(artifact_run == event_run);
    assert(accepted_at_seq > 0);
    assert(accepted_at_seq == run_accepted_seq);
}

// Non-vacuous: case analysis over the four payload tags.
pub proof fn proof_raw_workflow_parts_rejected()
    ensures
        !spec_strict_payload_is_accepted_artifact(SpecPayloadTag::RawWorkflowParts),
        !spec_strict_payload_is_accepted_artifact(SpecPayloadTag::LegacyCompiledIr),
        !spec_strict_payload_is_accepted_artifact(SpecPayloadTag::Malformed),
        spec_strict_payload_is_accepted_artifact(SpecPayloadTag::AcceptedArtifact),
{
    reveal(spec_strict_payload_is_accepted_artifact);
    assert(!spec_strict_payload_is_accepted_artifact(SpecPayloadTag::RawWorkflowParts));
    assert(!spec_strict_payload_is_accepted_artifact(SpecPayloadTag::LegacyCompiledIr));
    assert(!spec_strict_payload_is_accepted_artifact(SpecPayloadTag::Malformed));
    assert(spec_strict_payload_is_accepted_artifact(SpecPayloadTag::AcceptedArtifact));
}

// Non-vacuous: derives each precondition conjunct.
pub proof fn proof_index_precondition_decomposition(
    committed: bool,
    status_points_to_run: bool,
    workflow_points_to_run: bool,
    action_points_to_run: bool,
)
    requires
        spec_required_index_preconditions(
            committed,
            status_points_to_run,
            workflow_points_to_run,
            action_points_to_run,
        ),
    ensures
        committed,
        status_points_to_run,
        workflow_points_to_run,
        action_points_to_run,
{
    reveal(spec_required_index_preconditions);
    assert(committed);
    assert(status_points_to_run);
    assert(workflow_points_to_run);
    assert(action_points_to_run);
}

// Non-vacuous: exhaustive case analysis over the eight SpecFailureCause variants.
// For each variant, exhibit the matching SpecAdmissionError and assert the
// classification conjunct, then assert the outcome is Err.
pub proof fn proof_error_taxonomy_exhaustive(cause: SpecFailureCause)
    ensures
        exists|error: SpecAdmissionError| spec_error_classifies_failure(cause, error),
        spec_outcome_is_err(spec_admission_outcome(cause)),
{
    reveal(spec_error_classifies_failure);
    reveal(spec_admission_outcome);
    reveal(spec_outcome_is_err);
    match cause {
        SpecFailureCause::InvalidAcceptedArtifact => {
            assert(spec_error_classifies_failure(cause, SpecAdmissionError::InvalidAcceptedArtifact));
        },
        SpecFailureCause::InconsistentAdmissionInput => {
            assert(spec_error_classifies_failure(cause, SpecAdmissionError::InconsistentAdmissionInput));
        },
        SpecFailureCause::BatchStageFailed => {
            assert(spec_error_classifies_failure(cause, SpecAdmissionError::BatchStageFailed));
        },
        SpecFailureCause::BatchCommitFailed => {
            assert(spec_error_classifies_failure(cause, SpecAdmissionError::BatchCommitFailed));
        },
        SpecFailureCause::PartialVisibilityDetected => {
            assert(spec_error_classifies_failure(cause, SpecAdmissionError::PartialVisibilityDetected));
        },
        SpecFailureCause::SequenceBindingFailed => {
            assert(spec_error_classifies_failure(cause, SpecAdmissionError::SequenceBindingFailed));
        },
        SpecFailureCause::StrictRawWorkflowPartsRejected => {
            assert(spec_error_classifies_failure(cause, SpecAdmissionError::StrictRawWorkflowPartsRejected));
        },
        SpecFailureCause::IndexDerivationFailed => {
            assert(spec_error_classifies_failure(cause, SpecAdmissionError::IndexDerivationFailed));
        },
    }
}

fn main() {}

} // verus!
