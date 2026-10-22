#![forbid(unsafe_code)]
//! Durable retry end-to-end execution tests (vb-r5c1c).
//!
//! These tests use the real `Runtime` executor, the real `StorageRuntimeJournal`
//! over a real `FjallJournal`, and the real failure-boundary checkpoint producer
//! (not manually constructed checkpoints). A run fails its first attempt at an
//! action step, the failure boundary persists a full `RetryCheckpoint`, the journal
//! is cold-reopened, and `Runtime::recover_retry` re-executes the failed step.

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
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

fn shard_count() -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(1).ok_or_else(|| "shard count must be non-zero".to_string())
}

/// Computes the canonical content digest for a workflow: the `digest` field is
/// zeroed, the parts are postcard-serialized, and the result is BLAKE3-hashed.
/// This mirrors the storage admission checksum so the artifact is accepted.
fn canonical_workflow_digest(parts: &WorkflowParts) -> Result<WorkflowDigest, String> {
    let mut canonical = parts.clone();
    canonical.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let bytes =
        postcard::to_allocvec(&canonical).map_err(|e| format!("workflow encode failed: {e}"))?;
    Ok(WorkflowDigest::from_bytes(*blake3::hash(&bytes).as_bytes()))
}

fn retry_action_workflow() -> Result<CompiledWorkflow, String> {
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
        name: Box::from("retry-action"),
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
        .map_err(|e| format!("retry-action workflow invalid: {e:?}"))
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

fn latest_retry_checkpoint(events: &[JournalEvent]) -> Result<RunCheckpointV1, String> {
    let JournalEvent::RetryCheckpoint { payload, .. } = events
        .iter()
        .rev()
        .find(|e| matches!(e, JournalEvent::RetryCheckpoint { .. }))
        .ok_or_else(|| "no durable retry checkpoint in journal".to_string())?
    else {
        return Err("no durable retry checkpoint in journal".to_string());
    };
    postcard::from_bytes::<RunCheckpointV1>(payload)
        .map_err(|e| format!("checkpoint decode failed: {e}"))
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

/// First attempt fails at the action step, checkpoint is durably captured, then a
/// cold-reopened runtime re-executes the failed step and completes with the exact
/// result, leaving the prior journal prefix byte-identical.
#[test]
fn durable_retry_reopens_and_completes_failed_step() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();

    let workflow = retry_action_workflow()?;
    let contracts = contracts()?;
    let run = RunId::new(90_001);

    // ---- Scope A: produce the durable failure + checkpoint, then close ----
    let (checkpoint, next_seq, prefix_events) = {
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

        let counters = runtime.counters_snapshot();
        assert_eq!(counters.runs_failed, 1, "first attempt must fail");
        assert_eq!(counters.runs_completed, 0);

        let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, JournalEvent::RetryCheckpoint { .. })),
            "failure boundary must persist a durable retry checkpoint"
        );
        let checkpoint = latest_retry_checkpoint(&events)?;
        assert_eq!(checkpoint.run_id, run);
        assert_eq!(checkpoint.retry_step, StepIdx::ZERO);
        let next_seq = next_seq_after(&events)?;
        (checkpoint, next_seq, events)
    };

    // ---- Scope B: cold reopen and re-execute the failed step ----
    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    runtime
        .recover_retry(run, checkpoint, next_seq)
        .map_err(|e| format!("recover_retry failed: {e}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick failed: {e}"))?;

    // The failed step is re-executed: a new action ticket is scheduled for the
    // new run attempt.
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
        "retry must carry new run attempt number"
    );

    runtime
        .complete_action_with_output(retry_ticket, action_output(SlotValue::I64(42))?)
        .map_err(|e| format!("complete_action failed: {e}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick failed: {e}"))?;

    let counters = runtime.counters_snapshot();
    assert_eq!(counters.runs_completed, 1, "retry must complete the run");
    assert_eq!(counters.runs_failed, 0, "retry must not fail the run");

    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
    assert!(
        events.len() > prefix_events.len(),
        "retry must append events after the checkpoint"
    );
    assert_eq!(
        &events[..prefix_events.len()],
        &prefix_events[..],
        "prior journal prefix must be byte-identical after retry"
    );

    // The retry re-executes the failed action exactly once after the checkpoint.
    let scheduled = events
        .iter()
        .filter(|e| {
            matches!(e, JournalEvent::ActionScheduledTicket { ticket, .. }
            if ticket.run == run && ticket.step == StepIdx::ZERO)
        })
        .count();
    assert_eq!(
        scheduled, 2,
        "first attempt and retry attempt each schedule the action once"
    );

    let finished = events
        .iter()
        .any(|e| matches!(e, JournalEvent::RunFinished { attempt: 2, .. }));
    assert!(finished, "retry attempt must finish durably");
    Ok(())
}

/// The captured checkpoint carries the full resumable state: the next run attempt,
/// the failed step reset to Pending, the advanced per-action attempt counter, and a
/// prior sequence pointing at the audited `RunFailed` event.
#[test]
fn checkpoint_roundtrips_full_retry_authority() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();

    let workflow = retry_action_workflow()?;
    let contracts = contracts()?;
    let run = RunId::new(90_002);

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let artifact = vb_storage::submit_artifact_with_contracts(
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

    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
    let checkpoint = latest_retry_checkpoint(&events)?;

    assert_eq!(checkpoint.run_id, run);
    assert_eq!(
        checkpoint.run_attempt.get(),
        2,
        "checkpoint authorizes next attempt"
    );
    assert_eq!(checkpoint.retry_step, StepIdx::ZERO);
    assert_eq!(
        checkpoint.artifact_digest, artifact.digest,
        "checkpoint must bind the accepted artifact identity"
    );
    assert_eq!(checkpoint.workflow_digest, workflow.digest());

    // The failed step is reset to Pending and the PC is rewound to it; the
    // downstream Finish step remains Pending (never executed).
    assert_eq!(checkpoint.frame.pc, StepIdx::ZERO);
    assert_eq!(
        checkpoint.frame.states.as_slice(),
        [
            vb_core::frame::CheckpointStepState::Pending,
            vb_core::frame::CheckpointStepState::Pending
        ]
    );

    // The per-action attempt counter is captured verbatim (the failed step was
    // attempted once; the downstream step never).
    assert_eq!(checkpoint.action_attempts.as_ref(), [1, 0]);

    // The successful prefix (Do is step 0, nothing succeeded before it) and the
    // value store are captured verbatim; admission and contracts are retained.
    assert!(
        checkpoint.admission.is_some(),
        "admission authority retained"
    );
    assert_eq!(checkpoint.action_contracts.as_ref(), contracts.as_ref());

    // prior_seq points at the audited RunFailed event immediately before the checkpoint.
    let run_failed_seq = events
        .iter()
        .find(|e| matches!(e, JournalEvent::RunFailedEvent { .. }))
        .map(JournalEvent::seq)
        .ok_or_else(|| "no RunFailed event".to_string())?;
    assert_eq!(checkpoint.prior_seq, run_failed_seq.get());
    Ok(())
}
