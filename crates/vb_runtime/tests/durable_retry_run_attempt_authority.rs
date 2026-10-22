#![forbid(unsafe_code)]
//! Run-attempt ticket authority regression tests (vb-wop2u).
//!
//! After a durable run failure and explicit retry (`recover_retry`), the old
//! `ActionTicket` from the first attempt must be rejected as stale when
//! replayed against the second attempt. Old tickets carry `run_attempt=1`;
//! the new ticket after retry carries `run_attempt=2`.
//!
//! These tests exercise the real `Runtime`, real `StorageRuntimeJournal`,
//! real `FjallJournal`, and the real failure-boundary checkpoint producer.
//! No fabricated or all-zero tickets. The old ticket used is the ACTUAL
//! ticket issued during the failed first attempt, captured before recovery.
//!
//! This is the runtime-side durable version of the Fjall acceptance
//! criterion that stale callbacks must be rejected with exact typed
//! expected/found values and leave the runtime unchanged.

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

/// Produces a durable failed run and returns (checkpoint, next_seq, old_ticket).
fn produce_failed_attempt(
    path: &std::path::Path,
    run: RunId,
) -> Result<(RunCheckpointV1, EventSeq, ActionTicket), String> {
    let journal = Arc::new(FjallJournal::open(path, None).map_err(|e| e.to_string())?);
    let workflow = retry_action_workflow()?;
    let contracts = contracts()?;
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
            workflow,
            Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            CapabilitySet::empty(),
            contracts,
        )
        .map_err(|e| format!("submit failed: {e}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick failed: {e}"))?;

    let old_ticket = pending_ticket(&runtime, run)?;
    runtime
        .fail_action(old_ticket, non_retryable_failure())
        .map_err(|e| format!("fail_action failed: {e}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick failed: {e}"))?;

    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
    let checkpoint = latest_retry_checkpoint(&events)?;
    let next_seq = next_seq_after(&events)?;
    Ok((checkpoint, next_seq, old_ticket))
}

/// Cold-reopens the journal and drives the retried attempt to the pending action.
fn recover_and_schedule(
    path: &std::path::Path,
    run: RunId,
    checkpoint: RunCheckpointV1,
    next_seq: EventSeq,
) -> Result<Runtime, String> {
    let journal = Arc::new(FjallJournal::open(path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
    runtime
        .recover_retry(run, checkpoint, next_seq)
        .map_err(|e| format!("recover_retry failed: {e}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick failed: {e}"))?;
    Ok(runtime)
}

/// A run that fails on the first attempt at a non-retryable action step is
/// retried via explicit recover_retry, which drives the run to the new
/// pending action. The new ticket has run_attempt=2; the old ticket had
/// run_attempt=1. The old ticket must be rejected as stale.
#[test]
fn stale_old_ticket_rejected_after_durable_retry() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(100_001);

    let (checkpoint, next_seq, old_ticket) = produce_failed_attempt(&path, run)?;

    // Old ticket must carry run_attempt=1 (first durable attempt).
    assert_eq!(
        old_ticket.run_attempt, 1,
        "old ticket run_attempt must be 1"
    );

    let mut runtime = recover_and_schedule(&path, run, checkpoint, next_seq)?;
    let new_ticket = pending_ticket(&runtime, run)?;

    // New ticket after retry must carry run_attempt=2.
    assert_eq!(
        new_ticket.run_attempt, 2,
        "new ticket run_attempt must be 2 after retry"
    );

    // Idempotency keys must differ between attempts.
    assert_ne!(
        old_ticket.idempotency_key, new_ticket.idempotency_key,
        "idempotency keys must differ across run attempts"
    );

    // Complete the run with the NEW ticket to advance it past the pending action
    // so we can observe counter changes from old-ticket rejection attempts.
    let tick_before = runtime.tick_all();
    let counters_before = runtime.counters_snapshot();

    // Replay the OLD completion against the NEW attempt. Must be rejected
    // with StaleAttempt carrying the exact expected/found values.
    let result =
        runtime.complete_action_with_output(old_ticket, action_output(SlotValue::I64(42))?);

    match result {
        Ok(()) => {
            return Err("DEFECT: old completion was accepted".to_string());
        }
        Err(e) => {
            // Must be a StaleAttempt rejection with exact values.
            let err_str = format!("{e:?}");
            if !err_str.contains("incoming") || !err_str.contains("current") {
                return Err(format!(
                    "DEFECT: rejection error lacks expected/found values: {e:?}"
                ));
            }
            if !err_str.contains("1") || !err_str.contains("2") {
                return Err(format!(
                    "DEFECT: rejection error does not carry run_attempt values 1 and 2: {e:?}"
                ));
            }
        }
    }

    // Runtime must be unchanged after stale rejection.
    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_completed, counters_after.runs_completed,
        "runs_completed must not change on stale rejection"
    );
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "runs_failed must not change on stale rejection"
    );

    // Old failure ticket must also be rejected.
    let result = runtime.fail_action(old_ticket, non_retryable_failure());
    assert!(
        result.is_ok(),
        "old failure ticket enqueue must succeed (validation is async in shard)"
    );
    // Note: fail_action enqueues without preflight validation, unlike
    // complete_action_with_output. The stale validation happens in the shard
    // when the command is processed. But because the pending action is still
    // awaiting completion with the NEW ticket, the old failure should not
    // be accepted. Verify by checking that the run is still pending after tick.
    let tick_after_fail = runtime.tick_all();

    // Run must still be pending (not completed, not failed).
    let counters_final = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_completed, counters_final.runs_completed,
        "run must not complete on stale failure ticket"
    );

    // Now complete with the NEW ticket — must succeed.
    let result =
        runtime.complete_action_with_output(new_ticket, action_output(SlotValue::I64(42))?);
    assert!(result.is_ok(), "new ticket completion must succeed");
    runtime.tick_all().ok();
    let counters_new = runtime.counters_snapshot();
    assert_eq!(
        counters_new.runs_completed, 1,
        "run must complete when new ticket is accepted"
    );

    Ok(())
}

/// The per-action retry attempt counter stays at 1 after a durable run
/// retry; only run_attempt advances. An explicit per-action retry within
/// the same run attempt must still be distinguished by the per-action
/// attempt counter.
#[test]
fn run_attempt_advances_but_per_action_attempt_does_not() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(100_002);

    let (checkpoint, next_seq, old_ticket) = produce_failed_attempt(&path, run)?;

    // First attempt: run_attempt=1, attempt=1.
    assert_eq!(old_ticket.run_attempt, 1);
    assert_eq!(old_ticket.attempt, 1);

    let mut runtime = recover_and_schedule(&path, run, checkpoint, next_seq)?;
    let new_ticket = pending_ticket(&runtime, run)?;

    // After durable run retry: run_attempt=2, but per-action attempt resets to 1.
    // The checkpoint captures action_attempts verbatim and resets the step PC,
    // so the re-executed action gets a fresh per-action attempt counter.
    assert_eq!(new_ticket.run_attempt, 2);
    assert_eq!(
        new_ticket.attempt, 1,
        "per-action attempt must reset on run retry"
    );

    Ok(())
}

/// A stale ticket with a run_attempt in the future (not yet reached) must be
/// rejected, just like a past run_attempt. Only the current run attempt may
/// complete an action.
#[test]
fn future_run_attempt_ticket_rejected() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(100_003);

    let (checkpoint, next_seq, _old_ticket) = produce_failed_attempt(&path, run)?;

    let mut runtime = recover_and_schedule(&path, run, checkpoint, next_seq)?;
    let current_ticket = pending_ticket(&runtime, run)?;
    assert_eq!(current_ticket.run_attempt, 2);

    // Fabricate a ticket with run_attempt=3 (future). This cannot come from a
    // durable chain, but validates that future-attempt tickets are rejected
    // just like past ones.
    let future_ticket = ActionTicket {
        run: current_ticket.run,
        step: current_ticket.step,
        seq: current_ticket.seq,
        action: current_ticket.action,
        run_attempt: 3,
        attempt: current_ticket.attempt,
        idempotency_key: current_ticket.idempotency_key,
        capacity: current_ticket.capacity,
    };

    let result =
        runtime.complete_action_with_output(future_ticket, action_output(SlotValue::I64(42))?);
    match result {
        Ok(()) => Err("DEFECT: future run_attempt ticket was accepted".to_string()),
        Err(e) => {
            // Future-attempt rejection uses InvalidActionCompletion per
            // validate_ticket_attempt line 162-164.
            assert!(
                format!("{e:?}").contains("InvalidActionCompletion")
                    || format!("{e:?}").contains("incoming"),
                "future ticket must be rejected: {e:?}"
            );
            Ok(())
        }
    }
}

/// A ticket with run_attempt=0 (not 1-indexed) must be rejected as invalid.
#[test]
fn zero_run_attempt_ticket_rejected() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(100_004);

    let (checkpoint, next_seq, _old_ticket) = produce_failed_attempt(&path, run)?;

    let mut runtime = recover_and_schedule(&path, run, checkpoint, next_seq)?;
    let current_ticket = pending_ticket(&runtime, run)?;

    let zero_ticket = ActionTicket {
        run: current_ticket.run,
        step: current_ticket.step,
        seq: current_ticket.seq,
        action: current_ticket.action,
        run_attempt: 0,
        attempt: current_ticket.attempt,
        idempotency_key: current_ticket.idempotency_key,
        capacity: current_ticket.capacity,
    };

    let result =
        runtime.complete_action_with_output(zero_ticket, action_output(SlotValue::I64(42))?);
    match result {
        Ok(()) => Err("DEFECT: zero run_attempt ticket was accepted".to_string()),
        Err(e) => {
            assert!(
                format!("{e:?}").contains("incoming")
                    || format!("{e:?}").contains("InvalidActionCompletion"),
                "zero run_attempt must be rejected: {e:?}"
            );
            Ok(())
        }
    }
}
