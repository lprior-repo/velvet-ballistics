#![forbid(unsafe_code)]
//! Durable retry boundary regression tests (vb-hz9v8) for findings B, C, D.
//!
//! Every test drives the REAL public `Runtime` / `FjallJournal` boundary and the REAL
//! failure-boundary checkpoint producer; no manually-constructed checkpoints and no
//! fabricated all-zero events. The original adversarial probes for these findings live in
//! `.beads/vb-tf5lr/evidence/durable_retry_stale_probe.rs` (raw red repro preserved under
//! `.beads/vb-hz9v8/evidence/durable_retry_stale_probe_raw_red.rs`).
//!
//! Finding A (stale `ActionTicket`) is owned by LOCAL5090 `vb-wop2u` and is intentionally
//! not exercised here.

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

fn pending_ticket(runtime: &Runtime, run: RunId) -> Result<ActionTicket, String> {
    runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or_else(|| format!("run {run:?} has no pending action ticket"))
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

// =============================================================================
// Finding C — automatic cold recovery must not resume a terminal failed run
// =============================================================================

/// A failed run carrying a durable RetryCheckpoint is evidence-only until an explicit
/// `retry`: `recover_product` must classify it CannotResume and `recover_and_resume` must
/// reject it without mutating the journal.
#[test]
fn auto_recovery_does_not_resume_failed_run() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(92_004);

    let (_checkpoint, _next_seq, _old_ticket) = produce_failed_attempt(&path, run)?;

    // Cold reopen WITHOUT recover_retry: exercise the automatic recovery path.
    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let product = runtime
        .recover_product(run)
        .map_err(|e| format!("recover_product failed: {e}"))?;
    let resumable = product.is_resumable();

    let events_before = journal
        .events_for_run(run)
        .map_err(|e| e.to_string())?
        .len();
    let result = runtime.recover_and_resume(run);
    let tick = runtime.tick_all();
    let events_after = journal
        .events_for_run(run)
        .map_err(|e| e.to_string())?
        .len();

    assert!(
        !resumable,
        "recover_product must classify a terminal failed run as CannotResume"
    );
    assert!(
        matches!(product.cannot_resume_reason(), Some("terminal_state")),
        "reason must be terminal_state, got {:?}",
        product.cannot_resume_reason()
    );
    assert!(
        result.is_err(),
        "recover_and_resume must reject a terminal failed run, got {result:?}"
    );
    let _ = tick;
    assert_eq!(
        events_after, events_before,
        "automatic recovery must not mutate the journal"
    );
    Ok(())
}

/// A genuinely suspended run (pending action, no terminal evidence) must remain resumable
/// via the automatic path — the terminal gate must not over-reject the normal recovery case.
#[test]
fn auto_recovery_resumes_suspended_pending_action_run() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(92_005);

    // Produce a run suspended on a pending action (submit + tick, no failure).
    {
        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
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
        let _ = pending_ticket(&runtime, run)?;
    }

    // Cold reopen and exercise the automatic path on the suspended run.
    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let product = runtime
        .recover_product(run)
        .map_err(|e| format!("recover_product failed: {e:?}"))?;
    assert!(
        product.is_resumable(),
        "a suspended pending-action run must remain resumable"
    );
    Ok(())
}

// =============================================================================
// Finding D — raw recover_retry must bind checkpoint authority to the durable chain
// =============================================================================

/// A tampered `run_attempt` must be rejected before activation (no journal/state mutation).
#[test]
fn recover_retry_rejects_tampered_run_attempt() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(92_006);

    let (mut checkpoint, next_seq, _old_ticket) = produce_failed_attempt(&path, run)?;
    let events_before = {
        let journal = FjallJournal::open(&path, None).map_err(|e| e.to_string())?;
        journal
            .events_for_run(run)
            .map_err(|e| e.to_string())?
            .len()
    };
    checkpoint.run_attempt =
        core::num::NonZeroU16::new(99).ok_or_else(|| "99 is non-zero".to_string())?;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let result = runtime.recover_retry(run, checkpoint, next_seq);
    assert!(
        result.is_err(),
        "tampered run_attempt must be rejected, got {result:?}"
    );
    let _ = runtime.tick_all();
    let events_after = journal
        .events_for_run(run)
        .map_err(|e| e.to_string())?
        .len();
    assert_eq!(
        events_after, events_before,
        "rejected recover_retry must not mutate the journal"
    );
    Ok(())
}

/// A tampered `prior_seq` must be rejected (it is part of the bound checkpoint bytes).
#[test]
fn recover_retry_rejects_tampered_prior_seq() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(92_007);

    let (mut checkpoint, next_seq, _old_ticket) = produce_failed_attempt(&path, run)?;
    checkpoint.prior_seq = checkpoint
        .prior_seq
        .checked_add(7)
        .ok_or_else(|| "prior_seq overflow".to_string())?;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let result = runtime.recover_retry(run, checkpoint, next_seq);
    assert!(
        result.is_err(),
        "tampered prior_seq must be rejected, got {result:?}"
    );
    Ok(())
}

/// A `next_seq` inconsistent with the durable checkpoint position must be rejected.
#[test]
fn recover_retry_rejects_tampered_next_seq() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(92_008);

    let (checkpoint, next_seq, _old_ticket) = produce_failed_attempt(&path, run)?;
    let wrong_next_seq = EventSeq::new(
        next_seq
            .get()
            .checked_add(5)
            .ok_or_else(|| "next_seq overflow".to_string())?,
    );

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let result = runtime.recover_retry(run, checkpoint, wrong_next_seq);
    assert!(
        result.is_err(),
        "tampered next_seq must be rejected, got {result:?}"
    );
    Ok(())
}

/// A checkpoint whose `run_id` names a different run must be rejected.
#[test]
fn recover_retry_rejects_cross_run_checkpoint() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run_a = RunId::new(92_009);
    let run_b = RunId::new(92_010);

    let (checkpoint, next_seq, _old_ticket) = produce_failed_attempt(&path, run_a)?;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let result = runtime.recover_retry(run_b, checkpoint, next_seq);
    assert!(
        result.is_err(),
        "cross-run checkpoint must be rejected, got {result:?}"
    );
    Ok(())
}

/// A checkpoint applied to a run with no durable RetryCheckpoint must be rejected.
#[test]
fn recover_retry_rejects_missing_durable_checkpoint() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run_a = RunId::new(92_011);
    let run_b = RunId::new(92_012);

    let (mut checkpoint, next_seq, _old_ticket) = produce_failed_attempt(&path, run_a)?;
    // Re-target the checkpoint payload to run B, whose journal has no RetryCheckpoint.
    checkpoint.run_id = run_b;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let result = runtime.recover_retry(run_b, checkpoint, next_seq);
    assert!(
        result.is_err(),
        "a run with no durable RetryCheckpoint must be rejected, got {result:?}"
    );
    Ok(())
}

/// A stale checkpoint from an earlier attempt must not authorize a retry after a later
/// failure produced a newer checkpoint; the current checkpoint still must.
#[test]
fn recover_retry_rejects_stale_checkpoint_but_accepts_current() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(92_013);

    // First failure -> checkpoint_1 (run_attempt 2).
    let (checkpoint_1, next_seq_1, _old_ticket) = produce_failed_attempt(&path, run)?;

    // Explicit retry, then fail again -> checkpoint_2 (run_attempt 3).
    let (checkpoint_2, next_seq_2) = {
        let mut runtime = recover_and_schedule(&path, run, checkpoint_1.clone(), next_seq_1)?;
        let ticket = pending_ticket(&runtime, run)?;
        runtime
            .fail_action(ticket, non_retryable_failure())
            .map_err(|e| format!("fail_action failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;
        // Release the runtime (and its journal handle) before cold-reopening.
        drop(runtime);

        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
        let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
        let checkpoint = latest_retry_checkpoint(&events)?;
        let next_seq = next_seq_after(&events)?;
        (checkpoint, next_seq)
    };
    assert_eq!(checkpoint_2.run_attempt.get(), 3);

    // Stale checkpoint_1 must be rejected.
    {
        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
        let result = runtime.recover_retry(run, checkpoint_1, next_seq_1);
        assert!(
            result.is_err(),
            "stale checkpoint must be rejected, got {result:?}"
        );
    }

    // Current checkpoint_2 must be accepted and drive the failed step.
    {
        let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
        runtime
            .recover_retry(run, checkpoint_2, next_seq_2)
            .map_err(|e| format!("recover_retry failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;
        let ticket = pending_ticket(&runtime, run)?;
        assert_eq!(
            ticket.step,
            StepIdx::ZERO,
            "current checkpoint must re-drive the failed step"
        );
    }
    Ok(())
}

/// A checkpoint that was already consumed by an explicit retry (the run has since advanced
/// past it, here to completion) must be rejected even when the caller supplies the stale
/// original `next_seq` — the journal head, not the caller's `next_seq`, is authoritative.
#[test]
fn recover_retry_rejects_already_consumed_checkpoint() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(92_016);

    let (checkpoint, next_seq, _old_ticket) = produce_failed_attempt(&path, run)?;

    // Explicit retry, then complete the action so the run finishes at attempt 2.
    {
        let mut runtime = recover_and_schedule(&path, run, checkpoint.clone(), next_seq)?;
        let ticket = pending_ticket(&runtime, run)?;
        runtime
            .complete_action_with_output(ticket, action_output(SlotValue::I64(42))?)
            .map_err(|e| format!("complete_action failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;
        let counters = runtime.counters_snapshot();
        assert_eq!(counters.runs_completed, 1, "retry must complete the run");
    }

    // The same checkpoint + stale next_seq must now be rejected: the journal advanced past
    // the checkpoint (StepStarted/ActionCompleted/RunFinished), so it is already consumed.
    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let events_before = journal
        .events_for_run(run)
        .map_err(|e| e.to_string())?
        .len();
    let result = runtime.recover_retry(run, checkpoint, next_seq);
    assert!(
        result.is_err(),
        "already-consumed checkpoint must be rejected, got {result:?}"
    );
    let events_after = journal
        .events_for_run(run)
        .map_err(|e| e.to_string())?
        .len();
    assert_eq!(
        events_after, events_before,
        "rejected consumed checkpoint must not mutate the journal"
    );
    Ok(())
}

// =============================================================================
// Finding B — the retry runtime must use the strict durability barrier
// =============================================================================

/// `StorageRuntimeJournal::shared_strict` (the constructor `cmd_retry` now uses) must route
/// appends through the strict `append_strict` (SyncAll) barrier, i.e. each append must be
/// atomically durable and immediately visible. This pins the strict profile wiring that
/// finding B's fix relies on.
#[test]
fn shared_strict_journal_persists_each_append() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let run = RunId::new(92_014);
    let workflow = WorkflowDigest::from_bytes([42; 32]);

    shared
        .append_sequenced(
            vb_runtime::journal::RuntimeJournalEvent::RunSubmitted { run, workflow },
            EventSeq::new(0),
        )
        .map_err(|e| format!("strict append failed: {e}"))?;

    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
    assert_eq!(events.len(), 1, "strict append must be immediately durable");
    let event = events
        .first()
        .ok_or("strict append must produce exactly one durable event")?;
    assert!(matches!(
        event,
        JournalEvent::RunAccepted { run: r, seq, workflow: w }
            if *r == run && *seq == EventSeq::new(0) && *w == workflow
    ));
    Ok(())
}
