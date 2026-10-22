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
fn versioned_decoder_rejects_version_1_header_before_layout_decode() -> Result<(), String> {
    // Construct a payload with VARINT-encoded version 1 followed by garbage bytes.
    // The version-aware decoder must reject the legacy version header before
    // attempting to decode the checkpoint layout. This is an explicit version
    // boundary test, not a historical checkpoint migration or compatibility test.
    let payload = [1, 0, 0, 0];
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
    // Construct a payload with VARINT-encoded version 99 (unrecognized future).
    // The decoder must reject any version other than the currently supported 2.
    let payload = [99, 0, 0, 0];
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
fn versioned_decoder_rejects_truncated_valid_checkpoint() -> Result<(), String> {
    // Construct a real valid version 2 checkpoint, serialize it, then truncate
    // the valid bytes to test decoding of incomplete payload.
    let workflow = workflow().ok_or_else(|| "workflow compilation failed".to_string())?;
    let run_id = RunId::new(200);
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

    // Verify this is a version 2 checkpoint before serializing.
    assert_eq!(checkpoint.version, 2);

    // Serialize the valid checkpoint.
    let bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;

    // Verify the full valid payload decodes successfully.
    let full_decode = decode_checkpoint_payload(&bytes)
        .map_err(|error| format!("full decode should succeed: {error}"))?;
    assert_eq!(full_decode, checkpoint);

    // Now truncate the valid bytes by removing the last byte.
    let truncated = bytes.clone();
    let truncated_len = bytes.len()
        .checked_sub(1)
        .ok_or_else(|| "bytes length underflow".to_string())?;
    let mut truncated_bytes = truncated;
    truncated_bytes.truncate(truncated_len);

    // The truncated payload should fail to decode.
    let result = decode_checkpoint_payload(&truncated_bytes);
    match result {
        Err(RunCheckpointV1Error::DecodeError) => Ok(()),
        Err(other) => Err(format!("expected DecodeError for truncated payload, got {other}")),
        Ok(cp) => Err(format!(
            "expected failure for truncated payload, got checkpoint with version {}",
            cp.version
        )),
    }
}

#[test]
fn versioned_decoder_rejects_malformed_payload() -> Result<(), String> {
    // Version 2 header (VARINT 2) but garbage body that cannot decode as RunCheckpointV1.
    let payload = [
        2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
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
fn versioned_decoder_rejects_empty_payload() -> Result<(), String> {
    let result = decode_checkpoint_payload(&[]);
    match result {
        Err(RunCheckpointV1Error::UnsupportedVersion) => Ok(()),
        Err(other) => Err(format!("expected UnsupportedVersion for empty payload, got {other}")),
        Ok(cp) => Err(format!(
            "expected failure for empty payload, got checkpoint with version {}",
            cp.version
        )),
    }
}

#[test]
fn versioned_decoder_rejects_truncated_varint_version() -> Result<(), String> {
    // A payload where the VARINT version field itself is truncated.
    // VARINT values >= 128 require multiple bytes; byte 255 (high bit set)
    // signals a continuation byte that never arrives.
    let payload = [255u8];
    let result = decode_checkpoint_payload(&payload);
    match result {
        Err(RunCheckpointV1Error::UnsupportedVersion) => Ok(()),
        Err(other) => Err(format!(
            "expected UnsupportedVersion for truncated varint, got {other}"
        )),
        Ok(cp) => Err(format!(
            "expected failure for truncated varint, got checkpoint with version {}",
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

    // Verify the entire checkpoint is preserved, not just the policy pin.
    assert_eq!(decoded, checkpoint, "full checkpoint must roundtrip exactly");
    Ok(())
}
