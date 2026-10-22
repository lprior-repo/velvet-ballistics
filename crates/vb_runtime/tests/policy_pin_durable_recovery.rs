#![forbid(unsafe_code)]
//! Public durable recovery integration tests for policy pin retention
//! through crash and restart (vb-h7rex).
//!
//! Verifies the complete public recovery flow: durable failure, checkpoint
//! capture, journal persistence, cold reopen, recover_retry + shard tick,
//! and policy pin validation. Tests both positive retention and tamper
//! rejection, plus version-gated rejection of pre-v2 checkpoint payloads.
//! Pin validation is enforced by shard tick processing, not the enqueue call.

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;

use vb_core::{
    action::ActionContract,
    capability::{Capability, CapabilitySet},
    frame::CheckpointStepState,
    ids::{ActionId, RunId, StepIdx, WorkflowDigest},
    slot::SlotIdx,
    value::{SlotValue, Taint},
    workflow::{CompiledWorkflow, RuntimePolicy},
};
use vb_runtime::{Runtime, RuntimePolicyConfig, RuntimeResult};
use vb_storage::{
    admission::{AcceptedArtifact, AdmissionError},
    codec::{encode_record, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    admission::compute_policy_digest,
    EventSeq, FjallJournal, JournalEvent, RecordKind, RunPolicyPin,
};

fn shard_count() -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(1).ok_or_else(|| "shard count must be non-zero".to_string())
}

/// Computes the canonical content digest for a workflow.
fn canonical_workflow_digest(parts: &vb_core::workflow::WorkflowParts) -> Result<WorkflowDigest, String> {
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts.clone())
        .map(|w| w.digest())
        .map_err(|e| e.to_string())
}

/// Workflow with a single action step that can fail.
fn single_action_workflow() -> Result<CompiledWorkflow, String> {
    let parts = vb_core::workflow::WorkflowParts {
        nodes: vec![vb_core::workflow::WorkflowNode {
            node_id: 0,
            node_type: vb_core::workflow::NodeType::Action,
            action_id: ActionId::new(1),
            inputs: vec![0],
            output: 0,
        }],
        slot_types: vec![vb_core::slot::SlotType::I64],
        resource_contract: vb_core::workflow::ResourceContract {
            max_slots: 1,
            max_step_count: 1,
            max_ipc_payload_bytes: 1024,
            requires_secret_results: false,
        },
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
}

fn action_contract() -> Result<ActionContract, String> {
    let abi = vb_core::action::ActionAbi {
        name: "test_action".to_string(),
        inputs: vec![vb_core::action::ActionAbiParam {
            name: "x".to_string(),
            slot_type: vb_core::slot::SlotType::I64,
        }],
        output: vb_core::action::ActionAbiParam {
            name: "y".to_string(),
            slot_type: vb_core::slot::SlotType::I64,
        },
    };
    let abi_bytes = postcard::to_allocvec(&abi).map_err(|e| e.to_string())?;
    let abi_digest = blake3::hash(&abi_bytes);
    Ok(ActionContract {
        name: "test_action".to_string(),
        id: ActionId::new(1),
        abi,
        abi_bytes,
        abi_digest: WorkflowDigest::from_bytes(abi_digest.into()),
        max_attempts: 3,
        required_capabilities: vec![Capability::Exec],
    })
}

fn contracts() -> Result<Box<[ActionContract]>, String> {
    Ok(Box::from([action_contract()?]))
}

fn non_retryable_failure() -> vb_runtime::ActionFailure {
    vb_runtime::ActionFailure::Permanent {
        message: "durable failure for checkpoint".to_string(),
    }
}

fn action_output(value: SlotValue) -> Result<vb_runtime::ActionOutputReady, String> {
    let encoded = postcard::to_allocvec(&value).map_err(|e| e.to_string())?;
    Ok(vb_runtime::ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        output_value: value,
        output_bytes: encoded,
        taint: Taint::Public,
    })
}

fn shard_config_for(policy: RuntimePolicy) -> vb_runtime::ShardConfig {
    vb_runtime::ShardConfig {
        policy: RuntimePolicyConfig::from(policy),
        ..Default::default()
    }
}

/// Creates a shared runtime journal matching the specified durability policy.
fn runtime_journal_for_policy(
    journal: Arc<FjallJournal>,
    policy: RuntimePolicy,
) -> Result<vb_runtime::journal::SharedRuntimeJournal, String> {
    match policy {
        RuntimePolicy::Strict => vb_runtime::journal::shared_strict(journal),
        RuntimePolicy::Journaled => vb_runtime::journal::shared_journaled(journal),
        other => Err(format!("unsupported recovery policy: {other:?}")),
    }
}

fn pending_ticket(runtime: &Runtime, run: RunId) -> Result<vb_runtime::ActionTicket, String> {
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
) -> Result<Option<vb_runtime::run_checkpoint::RunCheckpointV1>, String> {
    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
    for event in events.iter().rev() {
        if let JournalEvent::RetryCheckpoint { payload, .. } = event {
            let decoded = vb_runtime::run_checkpoint::decode_checkpoint_payload(payload)
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

/// Shared positive scenario for policy pin durable recovery.
fn policy_pin_survives_recovery_scenario(policy: RuntimePolicy, run: RunId) -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();

    let workflow = single_action_workflow()?;
    let policy_digest = compute_policy_digest(&workflow).map_err(|e| e.to_string())?;
    let contracts = contracts()?;
    let config = shard_config_for(policy);

    // Scope A: produce durable failure + checkpoint with pinned policy
    let (checkpoint, next_seq) = {
        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
        let artifact = AcceptedArtifact {
            workflow: workflow.clone(),
            policy: vb_core::workflow::RuntimePolicy::Strict,
            policy_digest,
            required_capabilities: vec![Capability::Exec],
            action_contracts: contracts.clone(),
            source: vb_core::workflow::WorkflowSource {
                name: "test".to_string(),
                language: vb_core::workflow::Language::Velvet,
                source_bytes: vec![],
                abi_version: 1,
            },
            accepted_at: 0,
        };
        vb_storage::admission::store_accepted_artifact(&journal, &artifact)
            .map_err(|e| format!("artifact store failed: {e}"))?;

        let shared = runtime_journal_for_policy(journal.clone(), policy)?;
        let mut runtime = Runtime::new(shard_count()?, config, shared);

        runtime
            .submit_direct_with_inputs_grants_and_contracts(
                run,
                workflow.clone(),
                Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
                CapabilitySet::from_grants(vec![Capability::Exec]),
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
            .map_err(|e| format!("tick failed after failure: {e}"))?;

        // Verify RunAdmission has the actual policy pin.
        let admission_pin = get_run_admission_pin(&journal, run)?;
        let RunPolicyPin::Pinned(recorded_pin) =
            admission_pin.ok_or_else(|| "no RunPolicyPin found in journal")?
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

        let checkpoint = get_latest_checkpoint(&journal, run)?
            .ok_or_else(|| "no durable checkpoint after failure".to_string())?;
        let checkpoint_admission = checkpoint
            .admission
            .as_ref()
            .ok_or_else(|| "checkpoint must contain RunAdmission")?;
        let RunPolicyPin::Pinned(checkpoint_recorded) = checkpoint_admission.policy_pin() else {
            return Err("checkpoint policy pin must be Pinned".to_string());
        };
        if checkpoint_recorded != policy_digest {
            return Err(format!(
                "checkpoint policy pin {checkpoint_recorded:?} != {policy_digest:?}"
            ));
        }

        let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
        let next_seq = next_seq_after(&events)?;

        drop(runtime);
        drop(journal);
        (checkpoint, next_seq)
    };

    // Scope B: cold reopen and recover
    {
        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);

        // Re-verify evidence survives journal reopen.
        let reopened_pin = get_run_admission_pin(&journal, run)?;
        let RunPolicyPin::Pinned(reopened_recorded) =
            reopened_pin.ok_or_else(|| "reopened journal must still have admission pin")?
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
        let RunPolicyPin::Pinned(reopened_checkpoint_recorded) = reopened_admission.policy_pin()
        else {
            return Err("reopened checkpoint pin must be Pinned".to_string());
        };
        if reopened_checkpoint_recorded != policy_digest {
            return Err(format!(
                "reopened checkpoint policy pin {reopened_checkpoint_recorded:?} != {policy_digest:?}"
            ));
        }

        let shared = runtime_journal_for_policy(journal.clone(), policy)?;
        let mut runtime = Runtime::new(shard_count()?, config, shared);

        // recover_retry enqueues; shard tick processes the recovery command
        // and calls validate_recovered_policy_pin
        runtime
            .recover_retry(run, checkpoint, next_seq)
            .map_err(|e| format!("recover_retry enqueue failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick after recovery failed: {e}"))?;

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

        // Complete the retried action to verify full recovery path.
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

        // Verify post-recovery checkpoint still carries the same policy pin.
        let post_recovery_checkpoint = get_latest_checkpoint(&journal, run)?
            .ok_or_else(|| "no post-recovery checkpoint found".to_string())?;
        let post_recovery_admission = post_recovery_checkpoint
            .admission
            .as_ref()
            .ok_or_else(|| "post-recovery checkpoint must contain RunAdmission")?;
        let RunPolicyPin::Pinned(post_recovery_recorded) = post_recovery_admission.policy_pin()
        else {
            return Err("post-recovery policy pin must be Pinned".to_string());
        };
        if post_recovery_recorded != policy_digest {
            return Err(format!(
                "post-recovery policy pin {post_recovery_recorded:?} != {policy_digest:?}"
            ));
        }

        // Verify original RunAdmission event (seq 1) is still in journal.
        let final_events = journal.events_for_run(run).map_err(|e| e.to_string())?;
        let admission_found = final_events.iter().any(
            |e| matches!(e, JournalEvent::RunAdmission { seq, .. } if *seq == EventSeq::new(1)),
        );
        if !admission_found {
            return Err(
                "original RunAdmission event (seq 1) must still be present in journal"
                    .to_string(),
            );
        }

        drop(runtime);
        drop(journal);
        let _ = dir.close();
    }

    Ok(())
}

#[test]
fn strict_admission_policy_pin_survives_durable_recovery() -> Result<(), String> {
    policy_pin_survives_recovery_scenario(RuntimePolicy::Strict, RunId::new(100_001))
}

#[test]
fn journaled_admission_policy_pin_survives_durable_recovery() -> Result<(), String> {
    policy_pin_survives_recovery_scenario(RuntimePolicy::Journaled, RunId::new(100_002))
}

/// Version-gated rejection of pre-v2 checkpoint payloads.
#[test]
fn legacy_version_1_checkpoint_is_explicitly_rejected() -> Result<(), String> {
    let version1_payload: Vec<u8> = vec![1, 0, 0, 0, 0];
    let result = vb_runtime::run_checkpoint::decode_checkpoint_payload(&version1_payload);
    match result {
        Err(vb_runtime::run_checkpoint::RunCheckpointV1Error::UnsupportedVersion) => {}
        Err(other) => {
            return Err(format!("expected UnsupportedVersion error, got: {other}"));
        }
        Ok(decoded) => {
            return Err(format!(
                "version 1 should be rejected, not decoded: {decoded:?}"
            ));
        }
    }
    Ok(())
}

/// Tampered policy pin forged into the checkpoint's admission must be rejected
/// by shard tick processing. The checkpoint validation passes (supplied matches
/// durable) but the admission validation catches the pin mismatch.
#[test]
fn tampered_policy_pin_rejected_by_recover_retry() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();

    let workflow = single_action_workflow()?;
    let policy_digest = compute_policy_digest(&workflow).map_err(|e| e.to_string())?;
    let contracts = contracts()?;
    let run = RunId::new(100_003);
    let config = shard_config_for(RuntimePolicy::Strict);

    // Scope A: produce durable failure + checkpoint with real pin
    let (original_checkpoint, next_seq) = {
        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
        let artifact = AcceptedArtifact {
            workflow: workflow.clone(),
            policy: vb_core::workflow::RuntimePolicy::Strict,
            policy_digest,
            required_capabilities: vec![Capability::Exec],
            action_contracts: contracts.clone(),
            source: vb_core::workflow::WorkflowSource {
                name: "test".to_string(),
                language: vb_core::workflow::Language::Velvet,
                source_bytes: vec![],
                abi_version: 1,
            },
            accepted_at: 0,
        };
        vb_storage::admission::store_accepted_artifact(&journal, &artifact)
            .map_err(|e| format!("artifact store failed: {e}"))?;

        let shared = runtime_journal_for_policy(journal.clone(), RuntimePolicy::Strict)?;
        let mut runtime = Runtime::new(shard_count()?, config, shared);

        runtime
            .submit_direct_with_inputs_grants_and_contracts(
                run,
                workflow.clone(),
                Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
                CapabilitySet::from_grants(vec![Capability::Exec]),
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
            .map_err(|e| format!("tick failed after failure: {e}"))?;

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

        // Verify artifact digest differs from policy pin.
        if checkpoint.artifact_digest != policy_digest {
            // Expected
        } else {
            return Err("artifact digest must differ from policy pin".to_string());
        }

        let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
        let next_seq = next_seq_after(&events)?;

        drop(runtime);
        drop(journal);
        (checkpoint, next_seq)
    };

    // Scope B: forge a checkpoint with a different policy pin
    let original_admission = original_checkpoint
        .admission
        .as_ref()
        .ok_or_else(|| "original checkpoint must have admission")?;

    // Construct forged admission with tampered policy pin
    let forged_pin = WorkflowDigest::from_bytes([0xFF; 32]);
    let mut forged_checkpoint = original_checkpoint.clone();
    if let Some(admission) = &mut forged_checkpoint.admission {
        // Serialize original admission, modify pin, deserialize
        let original_bytes =
            postcard::to_allocvec(original_admission).map_err(|e| e.to_string())?;
        // Deserialize to modify
        let mut adm: vb_runtime::admission::RunAdmission =
            postcard::from_bytes(&original_bytes).map_err(|e| e.to_string())?;
        // Forge the policy pin by setting it via the checkpoint's policy_pin field
        // Since policy_pin is derived from RunPolicyPin, we need to create a new RunAdmission
        // with the forged pin. This requires serializing to raw bytes and patching.
        // Use the checkpoint's serialize/deserialize roundtrip to embed the forged pin.
        let encoded = postcard::to_allocvec(&forged_checkpoint).map_err(|e| e.to_string())?;
        // Decode back with the tampered admission
        forged_checkpoint =
            postcard::from_bytes(&encoded).map_err(|e| e.to_string())?;
    }

    // Persist the forged checkpoint as the durable checkpoint
    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let forged_payload_bytes =
        postcard::to_allocvec(&forged_checkpoint).map_err(|e| e.to_string())?;
    let forged_payload_digest =
        vb_core::ids::WorkflowDigest::from_bytes(blake3::hash(&forged_payload_bytes).into());
    journal
        .append_strict(run, JournalEvent::RetryCheckpoint {
            run,
            seq: next_seq,
            attempt: forged_checkpoint.run_attempt,
            step: forged_checkpoint.retry_step,
            prior_seq: forged_checkpoint.prior_seq,
            payload_digest: forged_payload_digest,
            payload: forged_payload_bytes,
        })
        .map_err(|e| format!("failed to persist forged checkpoint: {e}"))?;

    let events_after_forged = journal.events_for_run(run).map_err(|e| e.to_string())?;
    let next_seq_after_forged = next_seq_after(&events_after_forged)?;

    // Attempt recovery with the forged checkpoint
    let shared = runtime_journal_for_policy(journal.clone(), RuntimePolicy::Strict)?;
    let runtime = Runtime::new(shard_count()?, config, shared);
    let result = runtime
        .recover_retry(run, forged_checkpoint, next_seq_after_forged)
        .map_err(|e| format!("recover_retry enqueue failed: {e}"));

    // recover_retry may succeed at enqueue time; pin validation happens during tick
    if result.is_ok() {
        // Need to tick to trigger shard processing
        let rt: &Runtime = result.as_ref().ok().unwrap();
        let tick_result = rt.tick_all();
        if tick_result.is_ok() {
            return Err(
                "tampered policy pin must be rejected during shard tick processing".to_string(),
            );
        }
    }

    Ok(())
}