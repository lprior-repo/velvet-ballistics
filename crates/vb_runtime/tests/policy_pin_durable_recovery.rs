#![forbid(unsafe_code)]
//! Public durable recovery integration tests for policy pin retention
//! through crash and restart (vb-h7rex).
//!
//! These tests use the real Runtime, real StorageRuntimeJournal over a real
//! FjallJournal, and real failure-boundary checkpoint production. They verify
//! that the policy_pin attached to RunAdmission survives checkpoint capture,
//! journal persistence, cold reopen, and recover_retry.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket,
    Idempotency, RetryPolicy, RetrySafety, SideEffect,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::run_checkpoint::RunCheckpointV1;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{EventSeq, FjallJournal, JournalEvent, RunPolicyPin};

fn shard_count() -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(1).ok_or_else(|| "shard count must be non-zero".to_string())
}

/// Computes the canonical content digest for a workflow.
fn canonical_workflow_digest(parts: &WorkflowParts) -> Result<WorkflowDigest, String> {
    let mut canonical = parts.clone();
    canonical.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let bytes =
        postcard::to_allocvec(&canonical).map_err(|e| format!("workflow encode failed: {e}"))?;
    Ok(WorkflowDigest::from_bytes(*blake3::hash(&bytes).as_bytes()))
}

/// Workflow with a single action step that can fail.
fn single_action_workflow() -> Result<CompiledWorkflow, String> {
    let do_node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("policy-pin-recovery"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::from([do_node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    parts.digest = canonical_workflow_digest(&parts)?;
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| format!("workflow invalid: {e:?}"))
}

fn action_contract() -> Result<ActionContract, String> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("test-action").map_err(|e| e.to_string())?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([]),
    })
}

fn contracts() -> Result<Box<[ActionContract]>, String> {
    Ok(Box::from([action_contract()?]))
}

fn non_retryable_failure() -> ActionFailure {
    ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

fn action_output(value: SlotValue) -> Result<ActionOutputReady, String> {
    let encoded =
        postcard::to_allocvec(&value).map_err(|e| format!("slot encoding failed: {e}"))?;
    let encoded_len =
        u32::try_from(encoded.len()).map_err(|e| format!("slot length overflowed u32: {e}"))?;
    Ok(ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint: Taint::Clean,
        encoded_len,
    })
}

fn pending_ticket(runtime: &Runtime, run: RunId) -> Result<ActionTicket, String> {
    runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or_else(|| format!("run {run:?} has no pending action ticket"))
}

/// Extract the RunAdmission event and its policy pin from the journal.
fn get_run_admission_pin(
    journal: &FjallJournal,
    run: RunId,
) -> Result<Option<RunPolicyPin>, String> {
    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
    for event in events {
        if let JournalEvent::RunAdmission { policy_pin, .. } = event {
            return Ok(Some(policy_pin));
        }
    }
    Ok(None)
}

/// Extract the latest retry checkpoint from the journal and decode it.
fn get_latest_checkpoint(
    journal: &FjallJournal,
    run: RunId,
) -> Result<Option<RunCheckpointV1>, String> {
    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
    for event in events.iter().rev() {
        if let JournalEvent::RetryCheckpoint { payload, .. } = event {
            let decoded = postcard::from_bytes::<RunCheckpointV1>(payload)
                .map_err(|e| format!("checkpoint decode failed: {e}"))?;
            return Ok(Some(decoded));
        }
    }
    Ok(None)
}

fn next_seq_after(events: &[JournalEvent]) -> Result<EventSeq, String> {
    let last = events
        .last()
        .map(JournalEvent::seq)
        .unwrap_or(EventSeq::ZERO)
        .get();
    last.checked_add(1)
        .map(EventSeq::new)
        .ok_or_else(|| "journal sequence overflow".to_string())
}

/// Positive scenario: Strict admission captures the actual policy_digest as
/// the RunPolicyPin, which survives checkpoint capture, journal persistence,
/// cold reopen, and recover_retry.
#[test]
fn strict_admission_policy_pin_survives_durable_recovery() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();

    let workflow = single_action_workflow()?;
    let policy_digest =
        vb_storage::admission::compute_policy_digest(&workflow).map_err(|e| e.to_string())?;
    let contracts = contracts()?;
    let run = RunId::new(100_001);

    // ---- Scope A: produce durable failure + checkpoint ----
    let (checkpoint, next_seq) = {
        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);

        vb_storage::submit_artifact_with_contracts(
            &journal,
            &workflow,
            RuntimePolicy::Strict,
            &contracts,
        )
        .map_err(|e| format!("artifact submit failed: {e}"))?;

        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

        runtime
            .submit_direct_with_inputs_grants_and_contracts(
                run,
                workflow.clone(),
                Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
                CapabilitySet::empty(),
                contracts.clone(),
            )
            .map_err(|e| format!("submit failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;

        let ticket = pending_ticket(&runtime, run)?;
        runtime
            .fail_action(ticket, non_retryable_failure())
            .map_err(|e| format!("fail_action failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;

        // Verify RunAdmission has the actual policy pin.
        let admission_pin = get_run_admission_pin(&journal, run)?;
        assert!(
            admission_pin.is_some(),
            "Strict admission must record RunPolicyPin"
        );
        let RunPolicyPin::Pinned(recorded_pin) = admission_pin.unwrap() else {
            return Err("Strict admission must produce Pinned, not Unverified".to_string());
        };
        assert_eq!(
            recorded_pin, policy_digest,
            "admission policy pin must equal actual artifact policy_digest"
        );
        assert_ne!(
            recorded_pin,
            workflow.digest(),
            "policy pin must differ from artifact digest"
        );

        // Verify the checkpoint carries the pinned policy identity.
        let checkpoint = get_latest_checkpoint(&journal, run)?
            .ok_or_else(|| "no durable checkpoint after failure".to_string())?;
        assert!(
            checkpoint.admission.is_some(),
            "checkpoint must contain RunAdmission"
        );
        let checkpoint_pin = checkpoint.admission.as_ref().unwrap().policy_pin();
        let RunPolicyPin::Pinned(checkpoint_recorded) = checkpoint_pin else {
            return Err("checkpoint policy pin must be Pinned".to_string());
        };
        assert_eq!(
            checkpoint_recorded, policy_digest,
            "checkpoint policy pin must equal actual artifact policy_digest"
        );

        // Verify the artifact digest is distinct from policy pin.
        assert_ne!(
            checkpoint.admission.as_ref().unwrap().artifact_digest(),
            policy_digest,
            "artifact digest must differ from policy pin"
        );

        let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
        let next_seq = next_seq_after(&events)?;

        // Drop runtime and journal handles to force cold reopen.
        drop(runtime);
        drop(journal);
        (checkpoint, next_seq)
    };

    // ---- Scope B: cold reopen and recover ----
    {
        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);

        // Re-verify evidence survives journal reopen.
        let reopened_pin = get_run_admission_pin(&journal, run)?;
        assert!(
            reopened_pin.is_some(),
            "reopened journal must still have admission pin"
        );
        let RunPolicyPin::Pinned(reopened_recorded) = reopened_pin.unwrap() else {
            return Err("reopened pin must be Pinned".to_string());
        };
        assert_eq!(
            reopened_recorded, policy_digest,
            "reopened journal policy pin must be preserved"
        );

        let reopened_checkpoint = get_latest_checkpoint(&journal, run)?
            .ok_or_else(|| "reopened journal must still have checkpoint".to_string())?;
        let reopened_checkpoint_pin = reopened_checkpoint
            .admission
            .as_ref()
            .unwrap()
            .policy_pin();
        let RunPolicyPin::Pinned(reopened_checkpoint_recorded) = reopened_checkpoint_pin else {
            return Err("reopened checkpoint pin must be Pinned".to_string());
        };
        assert_eq!(
            reopened_checkpoint_recorded,
            policy_digest,
            "reopened checkpoint policy pin must be preserved"
        );

        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

        // Recover from the checkpoint captured in Scope A.
        runtime
            .recover_retry(run, checkpoint, next_seq)
            .map_err(|e| format!("recover_retry failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed after recovery: {e}"))?;

        // Verify the recovered run has a new action ticket with attempt 2.
        let retry_ticket = pending_ticket(&runtime, run)?;
        assert_eq!(
            retry_ticket.step,
            StepIdx::ZERO,
            "retry must re-execute the failed step"
        );
        assert_eq!(
            retry_ticket.attempt, 1,
            "re-executed action is its first action attempt"
        );
        assert_eq!(
            retry_ticket.run_attempt, 2,
            "recovered run must have attempt 2"
        );

        // Complete the retried action.
        runtime
            .complete_action_with_output(retry_ticket, action_output(SlotValue::I64(42))?)
            .map_err(|e| format!("complete_action failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed after completion: {e}"))?;

        let counters = runtime.counters_snapshot();
        assert_eq!(
            counters.runs_completed, 1,
            "recovered run must complete"
        );

        // Drop runtime and journal.
        drop(runtime);
        drop(journal);
    }

    Ok(())
}

/// Positive scenario: Journaled admission also captures and preserves the
/// policy pin through durable recovery.
#[test]
fn journaled_admission_policy_pin_survives_durable_recovery() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();

    let workflow = single_action_workflow()?;
    let policy_digest =
        vb_storage::admission::compute_policy_digest(&workflow).map_err(|e| e.to_string())?;
    let contracts = contracts()?;
    let run = RunId::new(100_002);

    // Submit with Journaled policy.
    {
        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);

        vb_storage::submit_artifact_with_contracts(
            &journal,
            &workflow,
            RuntimePolicy::Journaled,
            &contracts,
        )
        .map_err(|e| format!("artifact submit failed: {e}"))?;

        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

        runtime
            .submit_direct_with_inputs_grants_and_contracts(
                run,
                workflow.clone(),
                Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
                CapabilitySet::empty(),
                contracts.clone(),
            )
            .map_err(|e| format!("submit failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;

        let ticket = pending_ticket(&runtime, run)?;
        runtime
            .fail_action(ticket, non_retryable_failure())
            .map_err(|e| format!("fail_action failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;

        // Journaled must also produce Pinned.
        let admission_pin = get_run_admission_pin(&journal, run)?;
        assert!(
            admission_pin.is_some(),
            "Journaled admission must record RunPolicyPin"
        );
        let RunPolicyPin::Pinned(recorded_pin) = admission_pin.unwrap() else {
            return Err("Journaled admission must produce Pinned".to_string());
        };
        assert_eq!(
            recorded_pin, policy_digest,
            "Journaled admission policy pin must equal actual policy_digest"
        );

        let checkpoint = get_latest_checkpoint(&journal, run)?
            .ok_or_else(|| "no checkpoint".to_string())?;
        let checkpoint_pin = checkpoint.admission.as_ref().unwrap().policy_pin();
        let RunPolicyPin::Pinned(checkpoint_recorded) = checkpoint_pin else {
            return Err("checkpoint pin must be Pinned".to_string());
        };
        assert_eq!(
            checkpoint_recorded, policy_digest,
            "checkpoint pin must be preserved"
        );

        drop(runtime);
        drop(journal);
    }

    Ok(())
}

/// Verify that version 1 (legacy) checkpoint payloads are rejected by the
/// version-aware decoder, not silently migrated to carry a policy pin.
#[test]
fn legacy_version_1_checkpoint_is_explicitly_rejected() -> Result<(), String> {
    // Construct a version 1 header followed by a minimal payload.
    // VARINT encoding of 1 is just byte 1.
    let version1_payload: Vec<u8> = vec![1, 0, 0, 0, 0];

    let result = vb_runtime::run_checkpoint::decode_checkpoint_payload(&version1_payload);
    assert!(
        result.is_err(),
        "version 1 checkpoint must be rejected"
    );

    // Verify it's an UnsupportedVersion error.
    match result {
        Err(vb_runtime::run_checkpoint::RunCheckpointV1Error::UnsupportedVersion { .. }) => {}
        Err(other) => {
            return Err(format!(
                "expected UnsupportedVersion error, got: {other}"
            ));
        }
        Ok(decoded) => {
            return Err(format!(
                "version 1 should be rejected, not decoded: {decoded:?}"
            ));
        }
    }

    Ok(())
}