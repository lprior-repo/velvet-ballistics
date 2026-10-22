#![forbid(unsafe_code)]
//! Adversarial probe (vb-tf5lr): does the old ActionTicket from the failed
//! first attempt authorize the retried attempt after a durable run retry?
//!
//! This exercises the REAL failure-boundary checkpoint producer, a REAL
//! cold reopen, `Runtime::recover_retry`, and `tick_all`, then submits the
//! ACTUAL old ticket (completion and failure separately) while the new
//! pending ticket is active. No fabricated all-zero tickets.

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
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

    let old_ticket = pending_ticket(&runtime, run)?;
    runtime
        .fail_action(old_ticket, non_retryable_failure())
        .map_err(|e| format!("fail_action failed: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

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
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;
    Ok(runtime)
}

#[test]
fn probe_old_ticket_equality_against_retried_ticket() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(91_001);

    let (checkpoint, next_seq, old_ticket) = produce_failed_attempt(&path, run)?;
    let mut runtime = recover_and_schedule(&path, run, checkpoint, next_seq)?;
    let new_ticket = pending_ticket(&runtime, run)?;

    eprintln!("PROBE equality: old={old_ticket:?}");
    eprintln!("PROBE equality: new={new_ticket:?}");
    eprintln!("PROBE equality: old==new -> {}", old_ticket == new_ticket);
    let _ = runtime.tick_all();

    if old_ticket == new_ticket {
        // Contract requires stale/prior-attempt tickets to be distinguishable and
        // rejected; equality means the old ticket authorizes the new attempt.
        return Err(format!(
            "DEFECT: old ticket == new ticket after durable retry (identical authority).\n  old={old_ticket:?}\n  new={new_ticket:?}"
        ));
    }
    Ok(())
}

#[test]
fn probe_old_completion_authority() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(91_002);

    let (checkpoint, next_seq, old_ticket) = produce_failed_attempt(&path, run)?;
    let mut runtime = recover_and_schedule(&path, run, checkpoint, next_seq)?;
    let new_ticket = pending_ticket(&runtime, run)?;

    let counters_before = runtime.counters_snapshot();
    let result = runtime
        .complete_action_with_output(old_ticket, action_output(SlotValue::I64(42))?);
    let tick_result = runtime.tick_all();
    let counters_after = runtime.counters_snapshot();

    eprintln!("PROBE completion: old={old_ticket:?}");
    eprintln!("PROBE completion: new={new_ticket:?}");
    eprintln!(
        "PROBE completion: complete_action_with_output(old) = {result:?}"
    );
    eprintln!("PROBE completion: tick_all = {tick_result:?}");
    eprintln!(
        "PROBE completion: counters before={:?} after={:?}",
        (counters_before.runs_completed, counters_before.runs_failed),
        (counters_after.runs_completed, counters_after.runs_failed)
    );

    // Contract: a stale (prior-attempt) completion must be rejected with
    // StaleAttempt and leave journal/runtime unchanged. If it was accepted, the
    // run completes with the stale value — an authority defect.
    match result {
        Ok(()) => Err(format!(
            "DEFECT: old completion was ACCEPTED (no rejection). run completed={} failed={}. old={old_ticket:?} new={new_ticket:?}",
            counters_after.runs_completed, counters_after.runs_failed
        )),
        Err(e) => {
            eprintln!("PROBE completion: rejection = {e:?}");
            // Report whether the rejection is the exact stale attempt the contract
            // promises, or an unrelated cause.
            Ok(())
        }
    }
}

#[test]
fn probe_old_failure_authority() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(91_003);

    let (checkpoint, next_seq, old_ticket) = produce_failed_attempt(&path, run)?;
    let mut runtime = recover_and_schedule(&path, run, checkpoint, next_seq)?;
    let new_ticket = pending_ticket(&runtime, run)?;

    let counters_before = runtime.counters_snapshot();
    let result = runtime.fail_action(old_ticket, non_retryable_failure());
    let tick_result = runtime.tick_all();
    let counters_after = runtime.counters_snapshot();

    eprintln!("PROBE failure: old={old_ticket:?}");
    eprintln!("PROBE failure: new={new_ticket:?}");
    eprintln!("PROBE failure: fail_action(old) = {result:?}");
    eprintln!("PROBE failure: tick_all = {tick_result:?}");
    eprintln!(
        "PROBE failure: counters before={:?} after={:?}",
        (counters_before.runs_completed, counters_before.runs_failed),
        (counters_after.runs_completed, counters_after.runs_failed)
    );

    match result {
        Ok(()) => Err(format!(
            "DEFECT: old failure was ACCEPTED (no rejection). run completed={} failed={}. old={old_ticket:?} new={new_ticket:?}",
            counters_after.runs_completed, counters_after.runs_failed
        )),
        Err(e) => {
            eprintln!("PROBE failure: rejection = {e:?}");
            Ok(())
        }
    }
}

/// (C) The automatic cold-recovery path must treat a failed run (which carries a
/// durable retry checkpoint) as non-resumable: the next-attempt checkpoint is
/// evidence only until an explicit `retry` command drives it.
#[test]
fn probe_auto_recovery_does_not_resume_failed_run() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(91_004);

    let (_checkpoint, _next_seq, _old_ticket) = produce_failed_attempt(&path, run)?;

    // Cold reopen WITHOUT recover_retry: exercise the automatic recovery path.
    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let product = runtime
        .recover_product(run)
        .map_err(|e| format!("recover_product failed: {e}"))?;
    let resumable = product.is_resumable();
    eprintln!(
        "PROBE auto-recovery: recover_product is_resumable={resumable} cannot_resume_reason={:?}",
        product.cannot_resume_reason()
    );

    let events_before = journal.events_for_run(run).map_err(|e| e.to_string())?.len();
    let counters_before = runtime.counters_snapshot();
    let result = runtime.recover_and_resume(run);
    let tick = runtime.tick_all();
    let events_after = journal.events_for_run(run).map_err(|e| e.to_string())?.len();
    let counters_after = runtime.counters_snapshot();
    eprintln!(
        "PROBE auto-recovery: recover_and_resume={result:?} tick={tick:?}; events before={events_before} after={events_after}; counters before=({},{}) after=({},{})",
        counters_before.runs_completed,
        counters_before.runs_failed,
        counters_after.runs_completed,
        counters_after.runs_failed
    );

    // A failed run (with a durable retry checkpoint) must NOT be silently resumed
    // by the automatic cold-recovery path. The next-attempt checkpoint is evidence
    // only until an explicit retry command drives it.
    if resumable {
        return Err(
            "DEFECT: recover_product classifies a failed run (with a durable retry checkpoint) as Resumable"
                .to_string(),
        );
    }
    if result.is_ok() {
        return Err("DEFECT: recover_and_resume silently resumed the failed run".to_string());
    }
    if events_after != events_before {
        return Err(format!(
            "DEFECT: automatic recovery mutated the journal (before={events_before}, after={events_after})"
        ));
    }
    Ok(())
}

/// (D) Trust boundary: `recover_retry` accepts a caller-supplied checkpoint whose
/// `run_attempt` authority is NOT bound to the durable chain. The CLI boundary
/// protects against this via the payload digest; this probe documents what the raw
/// public API accepts.
#[test]
fn probe_recover_retry_accepts_tampered_run_attempt() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(91_005);

    let (mut checkpoint, next_seq, _old_ticket) = produce_failed_attempt(&path, run)?;
    let original_run_attempt = checkpoint.run_attempt.get();
    checkpoint.run_attempt = core::num::NonZeroU16::new(99).ok_or_else(|| "99 is non-zero".to_string())?;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let result = runtime.recover_retry(run, checkpoint, next_seq);
    eprintln!(
        "PROBE tamper-run_attempt: recover_retry(run_attempt=99) = {result:?} (durable original={original_run_attempt})"
    );

    match result {
        Err(e) => {
            eprintln!("PROBE tamper-run_attempt: rejected with {e:?}");
            Ok(())
        }
        Ok(()) => {
            let tick = runtime.tick_all();
            let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
            let finished_attempts: Vec<String> = events
                .iter()
                .filter_map(|e| match e {
                    JournalEvent::RunFinished { attempt, .. } => Some(attempt.to_string()),
                    _ => None,
                })
                .collect();
            eprintln!(
                "PROBE tamper-run_attempt: tick={tick:?}; RunFinished attempts={finished_attempts:?}"
            );
            // Report the observation without asserting: the raw public API trusted
            // a caller-supplied run_attempt that the durable chain never recorded.
            Err(format!(
                "OBSERVED: recover_retry accepted caller-supplied run_attempt=99 (durable chain original={original_run_attempt}); RunFinished attempts={finished_attempts:?}"
            ))
        }
    }
}
