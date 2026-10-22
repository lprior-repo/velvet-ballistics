use crate::admission::RunAdmission;
use crate::primitives::collect::CollectStates;
use crate::run_checkpoint::{RunCheckpointV1, RunCheckpointV1Error, decode_checkpoint_payload};
use crate::shard::types::RunState;
use core::num::NonZeroU16;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_storage::RunPolicyPin;

fn workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("retry-checkpoint"),
        digest: WorkflowDigest::from_bytes([3; 32]),
        semantic_source_identity: None,
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    })
    .ok()
}

fn failed_state(workflow: vb_core::workflow::CompiledWorkflow, run_id: RunId) -> Option<RunState> {
    let mut frame = RunFrame::new(run_id, StepIdx::ZERO, 1, 1).ok()?;
    frame.mark_running(StepIdx::ZERO).ok()?;
    frame.mark_failed(StepIdx::ZERO).ok()?;
    Some(RunState {
        registered_execution: crate::shard::types::RegisteredExecutionMode::Live,
        frame,
        run_attempt: NonZeroU16::MIN,
        workflow,
        store: ValueStore::new(),
        action_attempts: Box::from([0]),
        admission: Some(RunAdmission::new(
            WorkflowDigest::from_bytes([7; 32]),
            run_id,
            CapabilitySet::empty(),
            RuntimePolicy::Strict,
        )),
        collect_states: CollectStates::new(),
        action_contracts: Box::new([]),
    })
}

#[test]
fn capture_resets_only_retry_step_and_preserves_action_attempts() -> Result<(), String> {
    let workflow = workflow().ok_or_else(|| "workflow compilation failed".to_string())?;
    let run_id = RunId::new(41);
    let state =
        failed_state(workflow, run_id).ok_or_else(|| "failed state setup failed".to_string())?;
    let checkpoint = RunCheckpointV1::capture(
        run_id,
        WorkflowDigest::from_bytes([7; 32]),
        NonZeroU16::new(2).ok_or_else(|| "invalid attempt".to_string())?,
        StepIdx::ZERO,
        9,
        &state,
    )
    .map_err(|error| format!("capture failed: {error}"))?;

    assert_eq!(
        checkpoint.frame.states,
        [vb_core::frame::CheckpointStepState::Pending]
    );
    assert_eq!(checkpoint.frame.pc, StepIdx::ZERO);
    assert_eq!(checkpoint.action_attempts.as_ref(), [0]);
    assert_eq!(
        state.frame.step_state(StepIdx::ZERO),
        Ok(vb_core::frame::StepState::Failed)
    );
    assert_eq!(checkpoint.prior_seq, 9);
    let bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;
    let decoded: RunCheckpointV1 = postcard::from_bytes(&bytes)
        .map_err(|error| format!("checkpoint decode failed: {error}"))?;
    assert_eq!(decoded, checkpoint);
    let (restored, attempt, retry_step, prior_seq) = decoded
        .restore(
            state.workflow.clone(),
            WorkflowDigest::from_bytes([7; 32]),
            &state.action_contracts,
        )
        .map_err(|error| format!("checkpoint restore failed: {error}"))?;
    assert_eq!(attempt.get(), 2);
    assert_eq!(retry_step, StepIdx::ZERO);
    assert_eq!(prior_seq, 9);
    assert_eq!(
        restored.frame.step_state(StepIdx::ZERO),
        Ok(vb_core::frame::StepState::Pending)
    );
    assert_eq!(restored.action_attempts.as_ref(), [0]);
    Ok(())
}

#[test]
fn restore_rejects_a_checkpoint_for_a_different_workflow() -> Result<(), String> {
    let workflow = workflow().ok_or_else(|| "workflow compilation failed".to_string())?;
    let run_id = RunId::new(42);
    let state = failed_state(workflow.clone(), run_id)
        .ok_or_else(|| "failed state setup failed".to_string())?;
    let mut checkpoint = RunCheckpointV1::capture(
        run_id,
        WorkflowDigest::from_bytes([7; 32]),
        NonZeroU16::new(2).ok_or_else(|| "invalid attempt".to_string())?,
        StepIdx::ZERO,
        3,
        &state,
    )
    .map_err(|error| format!("capture failed: {error}"))?;
    checkpoint.workflow_digest = WorkflowDigest::from_bytes([9; 32]);
    assert_eq!(
        checkpoint.validate_against(
            &workflow,
            WorkflowDigest::from_bytes([7; 32]),
            &state.action_contracts,
        ),
        Err(RunCheckpointV1Error::WorkflowIdentityMismatch)
    );
    Ok(())
}

#[test]
fn versioned_decoder_rejects_historical_version_1_payload() -> Result<(), String> {
    // Construct a minimal payload with version 1 bytes (little-endian u16).
    // The rest is garbage; the version check must reject before attempting
    // to decode the RunAdmission layout that lacks policy_pin.
    let mut payload = [0u8; 4];
    payload[0] = 1;
    payload[1] = 0;
    let result = decode_checkpoint_payload(&payload);
    match result {
        Err(RunCheckpointV1Error::UnsupportedVersion) => Ok(()),
        Err(other) => Err(format!("expected UnsupportedVersion, got {other}")),
        Ok(cp) => Err(format!(
            "expected version rejection, got checkpoint with version {}",
            cp.version
        )),
    }
}

#[test]
fn versioned_decoder_rejects_unrecognized_future_version() -> Result<(), String> {
    let mut payload = [0u8; 4];
    payload[0] = 99;
    payload[1] = 0;
    let result = decode_checkpoint_payload(&payload);
    match result {
        Err(RunCheckpointV1Error::UnsupportedVersion) => Ok(()),
        Err(other) => Err(format!("expected UnsupportedVersion, got {other}")),
        Ok(cp) => Err(format!(
            "expected version rejection, got checkpoint with version {}",
            cp.version
        )),
    }
}

#[test]
fn versioned_decoder_rejects_truncated_payload() -> Result<(), String> {
    let payload = [0u8; 1];
    let result = decode_checkpoint_payload(&payload);
    match result {
        Err(RunCheckpointV1Error::UnsupportedVersion) => Ok(()),
        Err(other) => Err(format!("expected version decode failure, got {other}")),
        Ok(cp) => Err(format!(
            "expected failure, got checkpoint with version {}",
            cp.version
        )),
    }
}

#[test]
fn versioned_decoder_rejects_malformed_payload() -> Result<(), String> {
    // Version 2 header but garbage body that cannot decode as RunCheckpointV1.
    let mut payload = [0u8; 64];
    payload[0] = 2;
    payload[1] = 0;
    let result = decode_checkpoint_payload(&payload);
    match result {
        Err(RunCheckpointV1Error::DecodeError) => Ok(()),
        Err(other) => Err(format!("expected DecodeError, got {other}")),
        Ok(cp) => Err(format!(
            "expected decode failure, got checkpoint with version {}",
            cp.version
        )),
    }
}

#[test]
fn versioned_checkpoint_roundtrips_version_2_through_decoder() -> Result<(), String> {
    let workflow = workflow().ok_or_else(|| "workflow compilation failed".to_string())?;
    let run_id = RunId::new(101);
    let state =
        failed_state(workflow, run_id).ok_or_else(|| "failed state setup failed".to_string())?;
    let checkpoint = RunCheckpointV1::capture(
        run_id,
        WorkflowDigest::from_bytes([7; 32]),
        NonZeroU16::new(2).ok_or_else(|| "invalid attempt".to_string())?,
        StepIdx::ZERO,
        9,
        &state,
    )
    .map_err(|error| format!("capture failed: {error}"))?;

    // Verify the new checkpoint uses version 2.
    assert_eq!(checkpoint.version, 2);

    // Serialize through postcard and decode through the version-aware decoder.
    let bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;
    let decoded = decode_checkpoint_payload(&bytes)
        .map_err(|error| format!("version-aware decode failed: {error}"))?;
    assert_eq!(decoded, checkpoint);
    assert_eq!(decoded.version, 2);
    Ok(())
}

#[test]
fn versioned_decoder_preserves_pinned_admission_through_roundtrip() -> Result<(), String> {
    let workflow = workflow().ok_or_else(|| "workflow compilation failed".to_string())?;
    let run_id = RunId::new(102);
    let policy_digest = WorkflowDigest::from_bytes([11; 32]);
    let mut admission = RunAdmission::new(
        WorkflowDigest::from_bytes([7; 32]),
        run_id,
        CapabilitySet::empty(),
        RuntimePolicy::Strict,
    );
    admission = admission.with_policy_pin(RunPolicyPin::Pinned(policy_digest));

    let state =
        failed_state(workflow, run_id).ok_or_else(|| "failed state setup failed".to_string())?;
    let state_with_pin = RunState {
        admission: Some(admission),
        ..state
    };

    let checkpoint = RunCheckpointV1::capture(
        run_id,
        WorkflowDigest::from_bytes([7; 32]),
        NonZeroU16::new(2).ok_or_else(|| "invalid attempt".to_string())?,
        StepIdx::ZERO,
        15,
        &state_with_pin,
    )
    .map_err(|error| format!("capture failed: {error}"))?;

    // Verify the checkpoint captured the pinned admission.
    assert!(checkpoint.admission.is_some());
    let captured_pin = checkpoint.admission.as_ref().map(|a| a.policy_pin());
    assert_eq!(captured_pin, Some(RunPolicyPin::Pinned(policy_digest)));

    // Serialize and decode through the version-aware decoder.
    let bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;
    let decoded = decode_checkpoint_payload(&bytes)
        .map_err(|error| format!("version-aware decode failed: {error}"))?;

    // Verify the policy pin is preserved after the roundtrip.
    assert!(decoded.admission.is_some());
    let restored_pin = decoded.admission.as_ref().map(|a| a.policy_pin());
    assert_eq!(
        restored_pin,
        Some(RunPolicyPin::Pinned(policy_digest)),
        "policy pin must be preserved through version-aware decode roundtrip"
    );
    Ok(())
}
