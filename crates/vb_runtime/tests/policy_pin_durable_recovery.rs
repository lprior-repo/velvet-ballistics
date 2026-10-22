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
use vb_core::budget::AggregateResourceBudget;
use vb_runtime::admission::RunAdmission;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::run_checkpoint;
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

/// Test-local DTO with the same serde layout as RunAdmission, allowing
/// construction of a forged policy pin for tampered checkpoint tests.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ForgedRunAdmission {
    artifact_digest: WorkflowDigest,
    run_id: RunId,
    granted_capabilities: CapabilitySet,
    policy: RuntimePolicy,
    policy_pin: RunPolicyPin,
    budget: Option<AggregateResourceBudget>,
    idempotency_attested: Box<[ActionId]>,
}

fn forge_admission_with_pin(
    original: &RunAdmission,
    forged_pin: WorkflowDigest,
) -> RunAdmission {
    let dto = ForgedRunAdmission {
        artifact_digest: original.artifact_digest(),
        run_id: original.run_id(),
        granted_capabilities: original.granted_capabilities().clone(),
        policy: original.policy(),
        policy_pin: RunPolicyPin::Pinned(forged_pin),
        budget: None,
        idempotency_attested: Box::new([]),
    };
    let bytes =
        postcard::to_allocvec(&dto).expect("forged admission serialization must succeed");
    postcard::from_bytes::<RunAdmission>(&bytes).expect("forged admission deserialization must succeed")
}

fn shard_config_for(policy: RuntimePolicy) -> ShardConfig {
    ShardConfig {
        policy,
        ..ShardConfig::default()
    }
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
    for event in &events {
        if let JournalEvent::RunAdmission { policy_pin, .. } = event {
            return Ok(Some(*policy_pin));
        }
    }
    Ok(None)
}

/// Extract the latest retry checkpoint from the journal and decode it.
fn get_latest_checkpoint(
    journal: &FjallJournal,
    run: RunId,
) -> Result<Option<run_checkpoint::RunCheckpointV1>, String> {
    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
    for event in events.iter().rev() {
        if let JournalEvent::RetryCheckpoint { payload, .. } = event {
            let decoded =
                run_checkpoint::decode_checkpoint_payload(payload).map_err(|e| {
                    format!("checkpoint decode failed: {e}")
                })?;
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

/// Shared positive scenario for policy pin durable recovery.
/// Both Strict and Journaled must produce a pinned policy pin that survives
/// checkpoint capture, journal persistence, cold reopen, and recover_retry.
fn policy_pin_survives_recovery_scenario(
    policy: RuntimePolicy,
    run: RunId,
) -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();

    let workflow = single_action_workflow()?;
    let policy_digest =
        vb_storage::admission::compute_policy_digest(&workflow).map_err(|e| e.to_string())?;
    let contracts = contracts()?;

    // Build shard config with the specified policy.
    let config = shard_config_for(policy);

    // ---- Scope A: produce durable failure + checkpoint ----
    let (checkpoint, next_seq) = {
        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);

        // Submit the artifact first; strict/journaled policies require it.
        vb_storage::submit_artifact_with_contracts(
            &journal,
            &workflow,
            policy,
            &contracts,
        )
        .map_err(|e| format!("artifact submit failed: {e}"))?;

        // Create runtime with matching policy.
        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, config, shared);

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
        let RunPolicyPin::Pinned(recorded_pin) = admission_pin
            .ok_or_else(|| "no RunPolicyPin found in journal")?
        else {
            return Err("admission must produce Pinned, not Unverified".to_string());
        };
        if recorded_pin != policy_digest {
            return Err(format!(
                "admission policy pin {recorded_pin:?} != actual policy_digest {policy_digest:?}"
            ));
        }
        if recorded_pin == workflow.digest() {
            return Err("policy pin must differ from artifact digest".to_string());
        }

        // Verify the checkpoint carries the pinned policy identity.
        let checkpoint = get_latest_checkpoint(&journal, run)?
            .ok_or_else(|| "no durable checkpoint after failure".to_string())?;
        let admission = checkpoint
            .admission
            .as_ref()
            .ok_or_else(|| "checkpoint must contain RunAdmission")?;
        let RunPolicyPin::Pinned(checkpoint_recorded) = admission.policy_pin() else {
            return Err("checkpoint policy pin must be Pinned".to_string());
        };
        if checkpoint_recorded != policy_digest {
            return Err(format!(
                "checkpoint policy pin {checkpoint_recorded:?} != {policy_digest:?}"
            ));
        }

        // Verify the artifact digest is distinct from policy pin.
        if admission.artifact_digest() == policy_digest {
            return Err("artifact digest must differ from policy pin".to_string());
        }

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
        let RunPolicyPin::Pinned(reopened_recorded) = reopened_pin
            .ok_or_else(|| "reopened journal must still have admission pin")?
        else {
            return Err("reopened pin must be Pinned".to_string());
        };
        if reopened_recorded != policy_digest {
            return Err(format!(
                "reopened policy pin {reopened_recorded:?} != {policy_digest:?}"
            ));
        }

        let reopened_checkpoint = get_latest_checkpoint(&journal, run)?
            .ok_or_else(|| "reopened journal must still have checkpoint".to_string())?;
        let reopened_admission = reopened_checkpoint
            .admission
            .as_ref()
            .ok_or_else(|| "reopened checkpoint must contain RunAdmission")?;
        let RunPolicyPin::Pinned(reopened_checkpoint_recorded) = reopened_admission
            .policy_pin()
        else {
            return Err("reopened checkpoint pin must be Pinned".to_string());
        };
        if reopened_checkpoint_recorded != policy_digest {
            return Err(format!(
                "reopened checkpoint policy pin {reopened_checkpoint_recorded:?} != {policy_digest:?}"
            ));
        }

        // Create fresh runtime with matching policy for recovery.
        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, config, shared);

        // Recover from the checkpoint captured in Scope A.
        runtime
            .recover_retry(run, checkpoint, next_seq)
            .map_err(|e| format!("recover_retry failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed after recovery: {e}"))?;

        // Verify the recovered run has a new action ticket with attempt 2.
        let retry_ticket = pending_ticket(&runtime, run)?;
        if retry_ticket.step != StepIdx::ZERO {
            return Err(format!(
                "retry must re-execute the failed step; got step {}",
                retry_ticket.step.get()
            ));
        }
        if retry_ticket.attempt != 1 {
            return Err(format!(
                "re-executed action is its first action attempt; got attempt {}",
                retry_ticket.attempt
            ));
        }
        if retry_ticket.run_attempt != 2 {
            return Err(format!(
                "recovered run must have attempt 2; got run_attempt {}",
                retry_ticket.run_attempt
            ));
        }

        // Complete the retried action.
        runtime
            .complete_action_with_output(retry_ticket, action_output(SlotValue::I64(42))?)
            .map_err(|e| format!("complete_action failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed after completion: {e}"))?;

        let counters = runtime.counters_snapshot();
        if counters.runs_completed != 1 {
            return Err(format!(
                "recovered run must complete; runs_completed={}",
                counters.runs_completed
            ));
        }

        // After recovery, the new checkpoint produced by the retry must
        // still contain the same policy pin, proving retention through the
        // recovery path.
        let post_recovery_checkpoint = get_latest_checkpoint(&journal, run)?
            .ok_or_else(|| "no post-recovery checkpoint found".to_string())?;
        let post_recovery_admission = post_recovery_checkpoint
            .admission
            .as_ref()
            .ok_or_else(|| "post-recovery checkpoint must contain RunAdmission")?;
        let RunPolicyPin::Pinned(post_recovery_recorded) = post_recovery_admission
            .policy_pin()
        else {
            return Err("post-recovery policy pin must be Pinned".to_string());
        };
        if post_recovery_recorded != policy_digest {
            return Err(format!(
                "post-recovery policy pin {post_recovery_recorded:?} != {policy_digest:?}"
            ));
        }

        // Verify the journal preserves the complete historical prefix:
        // the original RunAdmission (seq 1) must still be present alongside
        // any new events.
        let final_events = journal.events_for_run(run).map_err(|e| e.to_string())?;
        let admission_found = final_events.iter().any(|e| {
            matches!(e, JournalEvent::RunAdmission { seq, .. } if *seq == EventSeq::new(1))
        });
        if !admission_found {
            return Err("original RunAdmission event must still be present in final journal".to_string());
        }

        drop(runtime);
        drop(journal);
    }

    Ok(())
}

/// Positive scenario: Strict admission captures the actual policy_digest as
/// the RunPolicyPin, which survives checkpoint capture, journal persistence,
/// cold reopen, and recover_retry.
#[test]
fn strict_admission_policy_pin_survives_durable_recovery() -> Result<(), String> {
    policy_pin_survives_recovery_scenario(
        RuntimePolicy::Strict,
        RunId::new(100_001),
    )
}

/// Positive scenario: Journaled admission also captures and preserves the
/// policy pin through durable recovery.
#[test]
fn journaled_admission_policy_pin_survives_durable_recovery() -> Result<(), String> {
    policy_pin_survives_recovery_scenario(
        RuntimePolicy::Journaled,
        RunId::new(100_002),
    )
}

/// Verify that version 1 (legacy) checkpoint payloads are rejected by the
/// version-aware decoder, not silently migrated to carry a policy pin.
#[test]
fn legacy_version_1_checkpoint_is_explicitly_rejected() -> Result<(), String> {
    // Construct a version 1 header followed by a minimal payload.
    // VARINT encoding of 1 is just byte 1.
    let version1_payload: Vec<u8> = vec![1, 0, 0, 0, 0];

    let result = run_checkpoint::decode_checkpoint_payload(&version1_payload);
    match result {
        Err(run_checkpoint::RunCheckpointV1Error::UnsupportedVersion) => {}
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

/// Negative scenario: a supplied checkpoint with a tampered policy pin
/// (forged to differ from the durable evidence) must be rejected by
/// recover_retry, preserving the original state and journal.
#[test]
fn tampered_policy_pin_rejected_by_recover_retry() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();

    let workflow = single_action_workflow()?;
    let policy_digest =
        vb_storage::admission::compute_policy_digest(&workflow).map_err(|e| e.to_string())?;
    let contracts = contracts()?;
    let run = RunId::new(100_003);

    // Build shard config for strict policy.
    let config = shard_config_for(RuntimePolicy::Strict);

    // ---- Scope A: produce durable failure + checkpoint with real pin ----
    let (original_checkpoint, next_seq) = {
        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);

        vb_storage::submit_artifact_with_contracts(
            &journal,
            &workflow,
            RuntimePolicy::Strict,
            &contracts,
        )
        .map_err(|e| format!("artifact submit failed: {e}"))?;

        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, config, shared);

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

        // Verify the real checkpoint has the correct pin.
        let checkpoint = get_latest_checkpoint(&journal, run)?
            .ok_or_else(|| "no durable checkpoint after failure".to_string())?;
        let admission = checkpoint
            .admission
            .as_ref()
            .ok_or_else(|| "checkpoint must contain RunAdmission")?;
        let RunPolicyPin::Pinned(recorded) = admission.policy_pin() else {
            return Err("checkpoint policy pin must be Pinned".to_string());
        };
        if recorded != policy_digest {
            return Err(format!(
                "checkpoint policy pin {recorded:?} != {policy_digest:?}"
            ));
        }

        let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
        let next_seq = next_seq_after(&events)?;

        drop(runtime);
        drop(journal);
        (checkpoint, next_seq)
    };

    // ---- Scope B: forge a checkpoint with a different policy pin ----
    // We use a test-local serde DTO to produce a hostile admission with
    // a tampered pin, then attach it to a clone of the real checkpoint.
    let original_admission = original_checkpoint
        .admission
        .as_ref()
        .ok_or_else(|| "original checkpoint must have admission")?;
    let forged_pin = WorkflowDigest::from_bytes([0xFF; 32]);
    let forged_admission = forge_admission_with_pin(original_admission, forged_pin);

    let mut forged_checkpoint = original_checkpoint.clone();
    forged_checkpoint.admission = Some(forged_admission);

    // Record the journal state before recovery to verify no mutation.
    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let events_before = journal.events_for_run(run).map_err(|e| e.to_string())?;
    let event_count_before = events_before.len();

    {
        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let runtime = Runtime::new(shard_count()?, config, shared);

        // The forged checkpoint should be rejected because its pin differs
        // from the durable evidence.
        let result = runtime.recover_retry(run, forged_checkpoint, next_seq);
        let Err(runtime_error) = result else {
            return Err("tampered policy pin must be rejected by recover_retry".to_string());
        };
        let error_debug = format!("{runtime_error:?}");
        if !error_debug
            .contains("retry checkpoint does not match durable RetryCheckpoint evidence")
        {
            return Err(format!(
                "expected evidence mismatch error, got: {error_debug}"
            ));
        }
    }

    // Verify the journal was not mutated by the failed recovery.
    let events_after = journal.events_for_run(run).map_err(|e| e.to_string())?;
    if events_after.len() != event_count_before {
        return Err(format!(
            "journal must not be mutated by rejected recovery; before={event_count_before}, after={}",
            events_after.len()
        ));
    }

    // Verify the original checkpoint still has the correct pin.
    let still_original = get_latest_checkpoint(&journal, run)?
        .ok_or_else(|| "checkpoint must still exist after rejected recovery".to_string())?;
    let still_original_admission = still_original
        .admission
        .as_ref()
        .ok_or_else(|| "original checkpoint admission must still exist")?;
    let RunPolicyPin::Pinned(still_recorded) = still_original_admission.policy_pin() else {
        return Err("original checkpoint pin must still be Pinned".to_string());
    };
    if still_recorded != policy_digest {
        return Err(format!(
            "original checkpoint pin must be unchanged; got {still_recorded:?}"
        ));
    }

    Ok(())
}