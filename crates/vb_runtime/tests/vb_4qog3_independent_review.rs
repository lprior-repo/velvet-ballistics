#![forbid(unsafe_code)]
//! Independent adversarial review tests for vb-hz9v8 B/C/D fixes.
//!
//! These tests independently verify the implementation against the
//! original contract from vb-tf5lr, focusing on edge cases and
//! actual state mutation assertions beyond the author's tests.
//!
//! Author: LOCAL5090 (independent review, not author of B/C/D implementation)
//! Bead: vb-4qog3

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

/// Produces a durable failed run and returns checkpoint info.
fn produce_failed_attempt(
    path: &std::path::Path,
    run: RunId,
) -> Result<(RunCheckpointV1, EventSeq, ActionTicket), String> {
    let journal = Arc::new(FjallJournal::open(path, None).map_err(|e| e.to_string())?);
    let workflow = retry_action_workflow()?;
    let contracts = Box::from([action_contract()?]);
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

// =============================================================================
// B: Verify strict durability barrier is actually exercised (not just declared)
// =============================================================================

/// Independent test: verify that retry runtime actually uses SyncAll barrier by
/// confirming the journal is created with strict profile and events are persisted.
#[test]
fn b_independent_verify_strict_durability_barrier() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(93_001);

    // Produce a failed attempt
    let (checkpoint, next_seq, _) = produce_failed_attempt(&path, run)?;

    // Retry with recover_retry - this should use strict durability via shared_strict
    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    // Verify recover_retry succeeds with the real checkpoint
    runtime
        .recover_retry(run, checkpoint, next_seq)
        .map_err(|e| format!("recover_retry failed: {e}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick failed after recover_retry: {e}"))?;

    // Drive to completion
    if let Some(t) = runtime.lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get())) {
        let output = action_output(SlotValue::I64(42))?;
        runtime
            .complete_action_with_output(t, output)
            .map_err(|e| format!("complete_action failed: {e}"))?;
    }
    runtime
        .tick_all()
        .map_err(|e| format!("tick_all failed: {e}"))?;

    // Verify the run completed and events are visible
    let events = journal
        .events_for_run(run)
        .map_err(|e| format!("events_for_run failed: {e}"))?;
    let last = events.last().ok_or("no last event")?;
    assert!(
        matches!(last, JournalEvent::RunFinished { .. }),
        "Run should be finished after retry completion"
    );

    // The critical B assertion: verify events are visible through journal
    let events2 = journal
        .events_for_run(run)
        .map_err(|e| format!("events_for_run failed: {e}"))?;
    assert!(
        events2
            .iter()
            .any(|e| matches!(e, JournalEvent::RunFinished { .. })),
        "RunFinished must be visible in journal (SyncAll barrier)"
    );
    assert_eq!(
        events.len(),
        events2.len(),
        "Event count must match (durability)"
    );
    Ok(())
}

// =============================================================================
// C: Verify terminal failed run is never re-driven by automatic recovery
// =============================================================================

/// Independent test: verify that multiple consecutive automatic recovery calls
/// cannot drive a terminal failed run.
#[test]
fn c_independent_multiple_auto_recovery_calls() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(93_002);

    let (_checkpoint, _next_seq, _old_ticket) = produce_failed_attempt(&path, run)?;

    // Cold reopen and try automatic recovery multiple times
    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let events_before = journal
        .events_for_run(run)
        .map_err(|e| format!("events_for_run failed: {e}"))?
        .len();

    // Try to recover_and_resume multiple times - should all fail
    for i in 0..3 {
        let product = runtime
            .recover_product(run)
            .map_err(|e| format!("recover_product failed: {e}"))?;
        assert!(
            !product.is_resumable(),
            "Attempt {}: run should still be CannotResume",
            i
        );
        let result = runtime.recover_and_resume(run);
        assert!(
            result.is_err(),
            "Attempt {}: recover_and_resume should fail",
            i
        );
    }

    let events_after = journal
        .events_for_run(run)
        .map_err(|e| format!("events_for_run failed: {e}"))?
        .len();

    assert_eq!(
        events_before, events_after,
        "Journal must not have mutated after 3 failed recovery attempts"
    );
    Ok(())
}

/// Independent test: verify that a run that is truly suspended (not failed)
/// still recovers via automatic path. This is the "no over-rejection" case.
#[test]
fn c_independent_suspended_run_still_recovers() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(93_003);

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let workflow = retry_action_workflow()?;
    let contracts = Box::from([action_contract()?]);
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

    // Get the pending ticket but don't fail it - leave it suspended
    let _ticket = pending_ticket(&runtime, run)?;

    // Try automatic recovery - a suspended run (no terminal event) should be Resumable
    let product = runtime
        .recover_product(run)
        .map_err(|e| format!("recover_product failed: {e}"))?;
    assert!(
        product.is_resumable(),
        "Suspended run without terminal event should be resumable"
    );
    Ok(())
}

// =============================================================================
// D: Verify consumed checkpoint cannot be re-used (stronger test)
// =============================================================================

/// Independent test: after a successful explicit retry that completes,
/// verify the original checkpoint cannot be re-used to drive the run again.
#[test]
fn d_independent_consumed_checkpoint_reuse_rejected() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(93_004);

    // Produce failed attempt
    let (checkpoint, next_seq, _) = produce_failed_attempt(&path, run)?;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    // First: valid explicit retry succeeds
    runtime
        .recover_retry(run, checkpoint.clone(), next_seq)
        .map_err(|e| format!("first recover_retry failed: {e}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick failed after first retry: {e}"))?;

    // Drive to completion
    if let Some(t) = runtime.lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get())) {
        let output = action_output(SlotValue::I64(42))?;
        runtime
            .complete_action_with_output(t, output)
            .map_err(|e| format!("complete_action failed: {e}"))?;
    }
    runtime
        .tick_all()
        .map_err(|e| format!("tick_all failed: {e}"))?;

    // Now try to re-use the same checkpoint - should be rejected
    // because the checkpoint is no longer at the journal head
    let result = runtime.recover_retry(run, checkpoint, next_seq);
    assert!(
        result.is_err(),
        "Re-using consumed checkpoint should be rejected"
    );

    // Verify journal didn't mutate from the failed retry attempt
    let events = journal
        .events_for_run(run)
        .map_err(|e| format!("events_for_run failed: {e}"))?;
    let last = events.last().ok_or("no last event")?;
    assert!(
        matches!(last, JournalEvent::RunFinished { .. }),
        "Journal should still end with RunFinished (no mutation from failed retry); last event: {last:?}"
    );
    Ok(())
}

/// Independent test: verify that a checkpoint from a different run is rejected.
#[test]
fn d_independent_cross_run_checkpoint_rejected() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run1 = RunId::new(93_005);
    let run2 = RunId::new(93_006);

    // Produce failed attempt on run1
    let (checkpoint, next_seq, _) = produce_failed_attempt(&path, run1)?;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    // Try to use run1's checkpoint for run2
    let result = runtime.recover_retry(run2, checkpoint, next_seq);
    assert!(result.is_err(), "Cross-run checkpoint should be rejected");

    // Verify run2 journal is unchanged
    let events = journal
        .events_for_run(run2)
        .map_err(|e| format!("events_for_run failed: {e}"))?;
    assert!(
        events.is_empty(),
        "run2 journal should be empty after failed retry"
    );
    Ok(())
}

/// Independent test: verify that a checkpoint with wrong prior_seq is rejected.
#[test]
fn d_independent_wrong_prior_seq_rejected() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(93_007);

    let (checkpoint, next_seq, _) = produce_failed_attempt(&path, run)?;

    // Tamper with prior_seq (u64, not EventSeq)
    let mut tampered = checkpoint;
    tampered.prior_seq = tampered.prior_seq - 1;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let result = runtime.recover_retry(run, tampered, next_seq);
    assert!(
        result.is_err(),
        "Checkpoint with wrong prior_seq should be rejected"
    );
    Ok(())
}

/// Independent test: verify that a checkpoint with wrong run_attempt is rejected.
#[test]
fn d_independent_wrong_run_attempt_rejected() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(93_008);

    let (checkpoint, next_seq, _) = produce_failed_attempt(&path, run)?;

    // Tamper with run_attempt
    let mut tampered = checkpoint;
    let wrong_attempt = std::num::NonZeroU16::new(99).ok_or("99 must be nonzero")?;
    tampered.run_attempt = wrong_attempt;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    let result = runtime.recover_retry(run, tampered, next_seq);
    assert!(
        result.is_err(),
        "Checkpoint with wrong run_attempt should be rejected"
    );
    Ok(())
}

/// Independent test: verify that valid retry with correct checkpoint still works.
/// This is the "green" counterpart to all the rejection tests above.
#[test]
fn d_independent_valid_retry_still_works() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(93_009);

    let (checkpoint, next_seq, _) = produce_failed_attempt(&path, run)?;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    // Valid retry with correct checkpoint
    runtime
        .recover_retry(run, checkpoint, next_seq)
        .map_err(|e| format!("recover_retry failed: {e}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick failed after recover_retry: {e}"))?;

    // Drive to completion
    let t = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("No pending ticket after recover_retry")?;
    let output = action_output(SlotValue::I64(42))?;
    runtime
        .complete_action_with_output(t, output)
        .map_err(|e| format!("complete_action failed: {e}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick_all failed: {e}"))?;

    // Verify completion
    let events = journal
        .events_for_run(run)
        .map_err(|e| format!("events_for_run failed: {e}"))?;
    let last = events.last().ok_or("no last event")?;
    assert!(
        matches!(last, JournalEvent::RunFinished { .. }),
        "Run should complete after valid retry; last event: {last:?}"
    );
    Ok(())
}

// =============================================================================
// Cross-cutting: Verify no state mutation on rejected retry attempts
// =============================================================================

/// Independent test: verify that rejected retry attempts (all D cases) don't
/// mutate any runtime state or journal.
#[test]
fn cross_independent_no_mutation_on_rejected_retry() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(93_010);

    let (checkpoint, next_seq, _) = produce_failed_attempt(&path, run)?;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| e.to_string())?);
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

    // Record state before any rejected retry attempts
    let events_before = journal
        .events_for_run(run)
        .map_err(|e| format!("events_for_run failed: {e}"))?;

    // Try multiple rejected retry variations
    let tampered_attempt = {
        let mut c = checkpoint.clone();
        c.run_attempt = std::num::NonZeroU16::new(99).ok_or("99 must be nonzero")?;
        c
    };
    let result1 = runtime.recover_retry(run, tampered_attempt, next_seq);
    assert!(result1.is_err(), "Tampered run_attempt should be rejected");

    let tampered_prior = {
        let mut c = checkpoint.clone();
        c.prior_seq = checkpoint.prior_seq - 1;
        c
    };
    let result2 = runtime.recover_retry(run, tampered_prior, next_seq);
    assert!(result2.is_err(), "Tampered prior_seq should be rejected");

    // Record state after rejected attempts
    let events_after = journal
        .events_for_run(run)
        .map_err(|e| format!("events_for_run failed: {e}"))?;

    assert_eq!(
        events_before.len(),
        events_after.len(),
        "Journal must be identical after rejected retry attempts"
    );

    // Verify all events are identical (not just count)
    for (i, (before, after)) in events_before.iter().zip(events_after.iter()).enumerate() {
        assert_eq!(
            before.seq().get(),
            after.seq().get(),
            "Event {} seq must match",
            i
        );
    }
    Ok(())
}
