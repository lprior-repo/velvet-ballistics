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
// This is a pure model. Fjall I/O, byte codecs, CLI formatting, runtime
// allocation, wall-clock time, and actual production structs are trusted shell
// boundaries that require later integration and formal-verifier evidence.
//
// BINDING: accepted_run_atomic_admission
// Rust type: vb_runtime::admission::RunAdmission
// Verified: Matched spec SpecPayloadTag/SpecFailureCause/SpecAdmissionOutcome to Rust RunAdmission fields
// Divergences: Spec models simplified error taxonomy; Rust uses actual error types from vb_runtime::admission

use vstd::prelude::*;

verus! {

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
    artifact_digest_matches
        && workflow_digest_matches
        && proof_matches
        && capability_set_matches
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
    same_run
        && same_workflow
        && spec_artifact_matches_header_and_source(
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

pub open spec fn spec_bind_accepted_at_seq(
    artifact_run: int,
    event_run: int,
    accepted_at_seq: int,
    run_accepted_seq: int,
) -> bool {
    artifact_run == event_run && accepted_at_seq > 0 && accepted_at_seq == run_accepted_seq
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
    committed && status_points_to_run && workflow_points_to_run && action_points_to_run
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
}

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
}

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
}

pub proof fn proof_raw_workflow_parts_rejected()
    ensures
        !spec_strict_payload_is_accepted_artifact(SpecPayloadTag::RawWorkflowParts),
        !spec_strict_payload_is_accepted_artifact(SpecPayloadTag::LegacyCompiledIr),
        !spec_strict_payload_is_accepted_artifact(SpecPayloadTag::Malformed),
        spec_strict_payload_is_accepted_artifact(SpecPayloadTag::AcceptedArtifact),
{
}

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
}

pub proof fn proof_error_taxonomy_exhaustive(cause: SpecFailureCause)
    ensures
        exists|error: SpecAdmissionError| spec_error_classifies_failure(cause, error),
        spec_outcome_is_err(spec_admission_outcome(cause)),
{
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

} // verus!

fn main() {}
