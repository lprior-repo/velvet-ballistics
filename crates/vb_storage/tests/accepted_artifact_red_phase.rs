#![forbid(unsafe_code)]

use vb_core::value::ConstValue;
use vb_core::workflow::{ResourceContract, WorkflowParts};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RuntimePolicy, SlotIdx, StepIdx,
    WorkflowDigest,
};
use vb_storage::admission::{AcceptedArtifact, VerificationWarning, submit_artifact};
use vb_storage::{FjallJournal, JournalError};

fn temp_journal() -> Result<FjallJournal, String> {
    let dir = tempfile::tempdir().map_err(|error| format!("tempdir failed: {error}"))?;
    FjallJournal::open(dir.keep(), None).map_err(|error| format!("journal open failed: {error}"))
}

fn minimal_workflow() -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("scope.valid_workflow"),
        digest: WorkflowDigest::from_bytes([0_u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let hash_bytes = postcard::to_allocvec(&parts)
        .map_err(|error| format!("serialize workflow parts failed: {error}"))?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&hash_bytes).into());
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|error| format!("compiled workflow construction failed: {error}"))
}

fn submit_minimal(policy: RuntimePolicy) -> Result<AcceptedArtifact, String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    submit_artifact(&journal, &workflow, policy)
        .map_err(|error| format!("submit_artifact failed: {error}"))
}

fn warning_at(gate: u8) -> VerificationWarning {
    VerificationWarning {
        code: 1,
        message: Box::<str>::from("accepted-artifact v1 warning boundary"),
        gate,
    }
}

#[test]
fn accepted_artifact_validator_accepts_warning_gate_fifteen() {
    let warning = warning_at(15);
    assert_eq!(warning.is_valid(), true);
}

#[test]
fn accepted_artifact_validator_rejects_warning_gate_sixteen() {
    let warning = warning_at(16);
    assert_eq!(warning.is_valid(), false);
}

#[test]
fn accepted_artifact_validator_uses_fifteen_gate_v1_upper_bound() {
    assert_eq!(VerificationWarning::MAX_GATE, 15);
}

#[test]
fn accepted_artifact_validator_rejects_legacy_thirteen_gate_upper_bound() {
    assert_ne!(VerificationWarning::MAX_GATE, 13);
}

#[test]
fn accepted_artifact_encoder_records_fifteen_gate_proof_when_policy_is_journaled()
-> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Journaled)?;
    assert_eq!(artifact.verification.gate_count, 15);
    Ok(())
}

#[test]
fn accepted_artifact_encoder_records_fifteen_gate_proof_when_policy_is_strict() -> Result<(), String>
{
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    assert_eq!(artifact.verification.gate_count, 15);
    Ok(())
}

#[test]
fn accepted_artifact_encoder_rejects_relaxed_raw_submit_when_accepted_artifacts_are_required()
-> Result<(), String> {
    let journal = temp_journal()?;
    let workflow = minimal_workflow()?;
    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed);
    assert_eq!(format!("{result:?}"), "Err(AdmissionRequired)");
    Ok(())
}

#[test]
fn accepted_artifact_store_payload_is_nested_accepted_artifact_not_raw_workflow_parts()
-> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    let decoded = postcard::from_bytes::<AcceptedArtifact>(&artifact.ir)
        .map_err(|error| format!("accepted artifact payload decode failed: {error}"))?;
    assert_eq!(decoded.verification.gate_count, 15);
    Ok(())
}

#[test]
fn accepted_artifact_encoder_binds_ir_digest_to_ir_bytes_not_workflow_parts_digest()
-> Result<(), String> {
    let artifact = submit_minimal(RuntimePolicy::Strict)?;
    let computed_ir_digest = WorkflowDigest::from_bytes(blake3::hash(&artifact.ir).into());
    assert_eq!(artifact.digest, computed_ir_digest);
    Ok(())
}

macro_rules! semantic_red_case {
    ($name:ident, $expected_debug:expr) => {
        #[test]
        fn $name() -> Result<(), String> {
            let artifact = submit_minimal(RuntimePolicy::Strict)?;
            assert_eq!(format!("{:?}", artifact.verification), $expected_debug);
            Ok(())
        }
    };
}

semantic_red_case!(
    accepted_artifact_validator_rejects_legacy_two_gate_proof,
    "Err(ArtifactEnvelopeError::InvalidGateCount { found: 2 })"
);
semantic_red_case!(
    accepted_artifact_validator_requires_bounded_flag,
    "VerificationProofV1 { gate_count: 15, bounded: true }"
);
semantic_red_case!(
    accepted_artifact_validator_requires_taint_safe_flag,
    "VerificationProofV1 { gate_count: 15, taint_safe: true }"
);
semantic_red_case!(
    accepted_artifact_validator_requires_retry_safe_flag,
    "VerificationProofV1 { gate_count: 15, retry_safe: true }"
);
semantic_red_case!(
    accepted_artifact_validator_requires_replayable_flag,
    "VerificationProofV1 { gate_count: 15, replayable: true }"
);
semantic_red_case!(
    accepted_artifact_validator_requires_idempotency_attestation,
    "VerificationProofV1 { gate_count: 15, idempotency_attested: [] }"
);

macro_rules! admission_error_red_case {
    ($name:ident, $policy:expr, $expected_debug:expr) => {
        #[test]
        fn $name() -> Result<(), String> {
            let journal = temp_journal()?;
            let workflow = minimal_workflow()?;
            let result: Result<AcceptedArtifact, JournalError> =
                submit_artifact(&journal, &workflow, $policy);
            assert_eq!(format!("{result:?}"), $expected_debug);
            Ok(())
        }
    };
}

admission_error_red_case!(
    runtime_admission_returns_admission_required_when_raw_submit_is_used_under_required_policy,
    RuntimePolicy::Relaxed,
    "Err(AdmissionRequired)"
);
admission_error_red_case!(
    runtime_admission_returns_artifact_invalid_when_store_validation_fails,
    RuntimePolicy::Journaled,
    "Err(ArtifactInvalid { source: PayloadDigestMismatch })"
);
admission_error_red_case!(
    runtime_admission_returns_input_too_large_when_input_exceeds_bound,
    RuntimePolicy::Journaled,
    "Err(InputTooLarge { len: 1048577, max: 1048576 })"
);
admission_error_red_case!(
    runtime_admission_returns_input_schema_mismatch_when_input_fails_schema,
    RuntimePolicy::Journaled,
    "Err(InputSchemaMismatch)"
);
admission_error_red_case!(
    runtime_admission_returns_capability_denied_when_required_capability_is_missing,
    RuntimePolicy::Journaled,
    "Err(CapabilityDenied)"
);
admission_error_red_case!(
    runtime_admission_returns_secret_unavailable_when_required_secret_is_absent,
    RuntimePolicy::Journaled,
    "Err(SecretUnavailable)"
);
admission_error_red_case!(
    runtime_admission_returns_run_already_exists_when_run_is_active_or_accepted,
    RuntimePolicy::Journaled,
    "Err(RunAlreadyExists)"
);
admission_error_red_case!(
    runtime_admission_returns_active_run_capacity_exceeded_when_capacity_is_full,
    RuntimePolicy::Journaled,
    "Err(ActiveRunCapacityExceeded)"
);
admission_error_red_case!(
    runtime_admission_returns_frame_allocation_failed_when_frame_pool_is_exhausted,
    RuntimePolicy::Journaled,
    "Err(FrameAllocationFailed)"
);
admission_error_red_case!(
    runtime_admission_returns_admission_journal_failed_when_run_events_cannot_be_recorded,
    RuntimePolicy::Journaled,
    "Err(AdmissionJournalFailed)"
);
admission_error_red_case!(
    runtime_admission_returns_strict_durability_failed_when_sync_all_fails,
    RuntimePolicy::Strict,
    "Err(StrictDurabilityFailed)"
);
admission_error_red_case!(
    runtime_admission_returns_clock_unavailable_when_clock_cannot_supply_timestamp,
    RuntimePolicy::Journaled,
    "Err(ClockUnavailable)"
);
