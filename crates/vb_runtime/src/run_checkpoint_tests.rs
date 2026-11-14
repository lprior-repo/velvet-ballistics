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
use vb_core::workflow::{CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_storage::RunPolicyPin;

fn workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let node = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
        None,
        None,
    );
    vb_core::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("retry-checkpoint"),
        digest: WorkflowDigest::from_bytes([3; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([vb_core::CompiledInputSlot {
            slot: vb_core::SlotIdx::new(0),
            kind: vb_core::InputSlotKind::Any,
            constraint: vb_core::CompiledInputConstraint::required(),
        }]),
    })
    .ok()
}

fn failed_state(workflow: vb_core::workflow::CompiledWorkflow, run_id: RunId) -> Option<RunState> {
    let mut frame = RunFrame::new(run_id, StepIdx::ZERO, 1, 1).ok()?;
    frame
        .write_slot(SlotIdx::ZERO, vb_core::SlotValue::I64(7))
        .ok()?;
    frame.mark_running(StepIdx::ZERO).ok()?;
    frame.mark_failed(StepIdx::ZERO).ok()?;
    let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    Some(RunState {
        registered_execution: crate::shard::types::RegisteredExecutionMode::Live,
        frame,
        run_attempt: NonZeroU16::MIN,
        workflow,
        store,
        action_attempts: Box::from([0]),
        admission: Some(RunAdmission::new(
            WorkflowDigest::from_bytes([7; 32]),
            run_id,
            CapabilitySet::empty(),
            RuntimePolicy::Strict,
        )),
        run_started_millis: None,
        collect_states: CollectStates::new(),
        journal_bytes_written: 0,
        action_contracts: Box::new([]),
    })
}

#[test]
fn capture_resets_only_retry_step_and_preserves_action_attempts() -> Result<(), String> {
    let workflow = workflow().ok_or_else(|| "workflow compilation failed".to_string())?;
    let run_id = RunId::new(41);
    let mut state =
        failed_state(workflow, run_id).ok_or_else(|| "failed state setup failed".to_string())?;
    state.journal_bytes_written = 1234;
    let checkpoint = RunCheckpointV1::capture(
        run_id,
        WorkflowDigest::from_bytes([7; 32]),
        NonZeroU16::new(2).ok_or_else(|| "invalid attempt".to_string())?,
        StepIdx::ZERO,
        9,
        &state,
    )
    .map_err(|error| format!("capture failed: {error}"))?;

    assert_eq!(checkpoint.journal_bytes_written, 1234);
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
    assert_eq!(restored.journal_bytes_written, 1234);
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
fn versioned_decoder_rejects_version_2_header_before_layout_decode() -> Result<(), String> {
    let payload = [2, 0, 0, 0];
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
    // The decoder must reject any version other than the currently supported 3.
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
    // Construct a real valid version 3 checkpoint, serialize it, then truncate
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

    // Verify this is a version 3 checkpoint before serializing.
    assert_eq!(checkpoint.version, 3);

    // Serialize the valid checkpoint.
    let bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;

    // Verify the full valid payload decodes successfully.
    let full_decode = decode_checkpoint_payload(&bytes)
        .map_err(|error| format!("full decode should succeed: {error}"))?;
    assert_eq!(full_decode, checkpoint);

    // Now truncate the valid bytes by removing the last byte.
    let truncated = bytes.clone();
    let truncated_len = bytes
        .len()
        .checked_sub(1)
        .ok_or_else(|| "bytes length underflow".to_string())?;
    let mut truncated_bytes = truncated;
    truncated_bytes.truncate(truncated_len);

    // The truncated payload should fail to decode.
    let result = decode_checkpoint_payload(&truncated_bytes);
    match result {
        Err(RunCheckpointV1Error::DecodeError) => Ok(()),
        Err(other) => Err(format!(
            "expected DecodeError for truncated payload, got {other}"
        )),
        Ok(cp) => Err(format!(
            "expected failure for truncated payload, got checkpoint with version {}",
            cp.version
        )),
    }
}

#[test]
fn versioned_decoder_rejects_malformed_payload() -> Result<(), String> {
    // Version 3 header (VARINT 3) but garbage body that cannot decode as RunCheckpointV1.
    let payload = [
        3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0,
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
        Err(other) => Err(format!(
            "expected UnsupportedVersion for empty payload, got {other}"
        )),
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
fn versioned_checkpoint_roundtrips_version_3_through_decoder() -> Result<(), String> {
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

    // Verify the new checkpoint uses version 3.
    assert_eq!(checkpoint.version, 3);

    // Serialize through postcard and decode through the version-aware decoder.
    let bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;
    let decoded = decode_checkpoint_payload(&bytes)
        .map_err(|error| format!("version-aware decode failed: {error}"))?;
    assert_eq!(decoded, checkpoint);
    assert_eq!(decoded.version, 3);
    Ok(())
}

#[test]
fn versioned_checkpoint_roundtrips_run_started_millis_through_decoder() -> Result<(), String> {
    let workflow = workflow().ok_or_else(|| "workflow compilation failed".to_string())?;
    let run_id = RunId::new(102);
    let mut state =
        failed_state(workflow, run_id).ok_or_else(|| "failed state setup failed".to_string())?;
    state.run_started_millis = Some(1_700_000_000_000);
    let checkpoint = RunCheckpointV1::capture(
        run_id,
        WorkflowDigest::from_bytes([7; 32]),
        NonZeroU16::new(2).ok_or_else(|| "invalid attempt".to_string())?,
        StepIdx::ZERO,
        9,
        &state,
    )
    .map_err(|error| format!("capture failed: {error}"))?;
    assert_eq!(checkpoint.run_started_millis, Some(1_700_000_000_000));
    let bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;
    let decoded = decode_checkpoint_payload(&bytes)
        .map_err(|error| format!("version-aware decode failed: {error}"))?;
    assert_eq!(decoded, checkpoint);
    assert_eq!(decoded.run_started_millis, Some(1_700_000_000_000));
    Ok(())
}

#[test]
fn versioned_decoder_rejects_version_2_header() -> Result<(), String> {
    let workflow = workflow().ok_or_else(|| "workflow compilation failed".to_string())?;
    let run_id = RunId::new(103);
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
    let mut bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;
    bytes[0] = 2;
    match decode_checkpoint_payload(&bytes) {
        Err(RunCheckpointV1Error::UnsupportedVersion) => Ok(()),
        Err(other) => Err(format!("expected UnsupportedVersion, got {other}")),
        Ok(cp) => Err(format!(
            "expected version rejection, got checkpoint with version {}",
            cp.version
        )),
    }
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
    let captured_pin = checkpoint.admission.as_ref().map(|a| a.policy_pin());
    assert_eq!(captured_pin, Some(RunPolicyPin::Pinned(policy_digest)));

    // Serialize and decode through the version-aware decoder.
    let bytes = postcard::to_allocvec(&checkpoint)
        .map_err(|error| format!("checkpoint encode failed: {error}"))?;
    let decoded = decode_checkpoint_payload(&bytes)
        .map_err(|error| format!("version-aware decode failed: {error}"))?;

    // Verify the policy pin is preserved after the roundtrip.
    let restored_pin = decoded.admission.as_ref().map(|a| a.policy_pin());
    assert_eq!(
        restored_pin,
        Some(RunPolicyPin::Pinned(policy_digest)),
        "policy pin must be preserved through version-aware decode roundtrip"
    );

    // Verify the entire checkpoint is preserved, not just the policy pin.
    assert_eq!(
        decoded, checkpoint,
        "full checkpoint must roundtrip exactly"
    );
    Ok(())
}

#[test]
fn nested_checkpoint_preserves_full_bound_admission() -> Result<(), String> {
    let workflow = workflow().ok_or_else(|| "workflow compilation failed".to_string())?;
    let run_id = RunId::new(500);
    let mut state = failed_state(workflow.clone(), run_id)
        .ok_or_else(|| "failed state setup failed".to_string())?;

    let artifact_digest = WorkflowDigest::from_bytes([7; 32]);
    let input_digest = vb_core::ids::WorkflowDigest::from_bytes([0x55; 32]);
    let budget = vb_core::budget::AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 60,
        max_result_bytes: 1000,
        max_total_slots_written: 10,
        max_timer_entries: 1,
        max_trace_events: 10,
        max_queue_depth: 10,
        max_journal_batch_bytes: 1024,
        max_ipc_payload_bytes: 1024,
        max_blob_bytes: 1000,
        max_input_bytes: 1000,
        max_step_budget_per_tick: 10,
        max_transitions_per_tick: 10,
        max_open_gates: 1,
        max_signal_subscriptions: 1,
        max_incident_emissions: 1,
        max_transitions: 100,
        max_journal_bytes_total: 10240,
        max_value_store_entries: 10,
        max_cost_units: 100,
    };

    let compat_inputs =
        vb_storage::compat::artifact_ir_version_inputs(vb_storage::compat::CURRENT_IR_VERSION);
    let decision = vb_storage::compat::decide_boundary(
        compat_inputs,
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION),
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION),
    );

    let full_admission = RunAdmission::new(
        artifact_digest,
        run_id,
        state.admission.clone().map_or_else(
            || vb_core::capability::CapabilitySet::empty(),
            |a| a.granted_capabilities().clone(),
        ),
        vb_core::policy::RuntimePolicy::Strict,
    )
    .with_input_digest(vb_storage::RunInputBinding::Digest(input_digest))
    .with_attached_budget(budget)
    .with_tenant(vb_core::ids::TenantId::new(42))
    .with_policy_pin(vb_storage::RunPolicyPin::Pinned(artifact_digest))
    .with_ir_compat_decision(decision);

    state.admission = Some(full_admission.clone());

    let checkpoint = RunCheckpointV1::capture(
        run_id,
        artifact_digest,
        NonZeroU16::new(1).ok_or_else(|| "invalid attempt".to_string())?,
        StepIdx::ZERO,
        0,
        &state,
    )
    .map_err(|e| format!("capture failed: {e}"))?;

    assert_eq!(checkpoint.version, 3, "checkpoint version should be 3");
    assert_eq!(
        checkpoint.admission.as_ref().map(|a| a.input_digest()),
        Some(vb_storage::RunInputBinding::Digest(input_digest)),
        "checkpoint should have bound input digest"
    );
    assert_eq!(
        checkpoint.admission.as_ref().and_then(|a| a.budget()),
        Some(budget),
        "checkpoint should have budget"
    );
    assert_eq!(
        checkpoint.admission.as_ref().map(|a| a.tenant()),
        Some(Some(vb_core::ids::TenantId::new(42))),
        "checkpoint should have tenant"
    );

    let bytes =
        postcard::to_allocvec(&checkpoint).map_err(|e| format!("checkpoint encode failed: {e}"))?;
    let decoded: RunCheckpointV1 =
        postcard::from_bytes(&bytes).map_err(|e| format!("checkpoint decode failed: {e}"))?;

    assert_eq!(
        decoded, checkpoint,
        "decoded checkpoint should match original"
    );

    let (restored, _, _, _) = decoded
        .restore(workflow.clone(), artifact_digest, &state.action_contracts)
        .map_err(|e| format!("restore failed: {e}"))?;

    // Complete admission comparison
    assert_eq!(
        restored.admission,
        Some(full_admission),
        "restored admission should be complete and identical to original"
    );

    Ok(())
}
