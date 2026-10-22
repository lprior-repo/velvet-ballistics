#![forbid(unsafe_code)]
//! Cross-crate ticket authority probe test.
//!
//! Validates run-attempt ticket authority, stale completion rejection, and
//! ticket serialization roundtrips through the real Runtime/Fjall public API.
//! Strengthened with full serialized journal vector captures and exact error
//! type assertions.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket,
    Idempotency, RetrySafety, SideEffect,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
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
        name: Box::from("test-retry"),
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
    CompiledWorkflow::try_from_parts(parts).map_err(|e| format!("workflow invalid: {e:?}"))
}

fn action_contract() -> Result<ActionContract, String> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("test-action").map_err(|e| format!("action name: {e}"))?,
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
        code: ActionFailureCode::Unknown,
        detail: None,
        retry_policy: vb_core::action::RetryPolicy::NonRetryable,
        encoded_len: 0,
        taint: Taint::Clean,
    }
}

fn action_output(value: SlotValue) -> Result<ActionOutputReady, String> {
    let encoded = postcard::to_allocvec(&value).map_err(|e| format!("encode failed: {e}"))?;
    let encoded_len =
        u32::try_from(encoded.len()).map_err(|_| "encoded length exceeds u32".to_string())?;
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
        .ok_or_else(|| "run has no pending action ticket".to_string())
}

fn latest_retry_checkpoint(events: &[JournalEvent]) -> Result<RunCheckpointV1, String> {
    for event in events.iter().rev() {
        if let JournalEvent::RetryCheckpoint { payload, .. } = event {
            let checkpoint: RunCheckpointV1 = postcard::from_bytes(payload)
                .map_err(|e| format!("checkpoint decode failed: {e}"))?;
            return Ok(checkpoint);
        }
    }
    Err("no retry checkpoint found".to_string())
}

fn next_seq_after(events: &[JournalEvent]) -> Result<EventSeq, String> {
    let max = events.iter().map(|e| e.seq()).max();
    let next = max.map(|s| s.get().saturating_add(1)).unwrap_or(0);
    Ok(EventSeq::new(next))
}

/// Serialize a vector of JournalEvent records for byte-equality comparison.
/// The entire slice is encoded as one postcard value so record boundaries are
/// preserved. Both before and after snapshots must use the same encoding.
fn serialize_event_vector(events: &[JournalEvent]) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(events).map_err(|e| format!("event vector serialize failed: {e}"))
}

/// Test: stale completion is rejected after 2+ retries with no state mutation.
///
/// Full serialized JournalEvent vectors are captured before/after stale rejection
/// to detect missing frames or lost state. The exact queued failure tick rejection
/// is asserted, then the current ticket is ticked and RunFinished is verified.
#[test]
fn stale_completion_rejected_after_two_retries() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir failed: {e}"))?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(90001);

    // First attempt: fail and checkpoint. Retain actual first-run ticket.
    let (checkpoint1, next_seq1, first_ticket) = {
        let journal =
            Arc::new(FjallJournal::open(&path, None).map_err(|e| format!("open journal: {e}"))?);
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

        let ticket = pending_ticket(&runtime, run)?;
        assert_eq!(
            ticket.run_attempt, 1,
            "first attempt must have run_attempt=1"
        );

        runtime
            .fail_action(ticket, non_retryable_failure())
            .map_err(|e| format!("fail_action failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;

        let events = journal
            .events_for_run(run)
            .map_err(|e| format!("events failed: {e}"))?;
        let checkpoint = latest_retry_checkpoint(&events)?;
        let next_seq = next_seq_after(&events)?;
        Ok::<(RunCheckpointV1, EventSeq, ActionTicket), String>((checkpoint, next_seq, ticket))
    }?;
    // ActionTicket is Copy; use retained first ticket directly as the stale ticket.

    // Second attempt: fail and checkpoint again
    let (checkpoint2, next_seq2) = {
        let journal =
            Arc::new(FjallJournal::open(&path, None).map_err(|e| format!("open journal: {e}"))?);
        let _workflow = retry_action_workflow()?;
        let _contracts = contracts()?;
        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

        runtime
            .recover_retry(run, checkpoint1, next_seq1)
            .map_err(|e| format!("recover_retry failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;

        let ticket = pending_ticket(&runtime, run)?;
        assert_eq!(
            ticket.run_attempt, 2,
            "second attempt must have run_attempt=2"
        );

        runtime
            .fail_action(ticket, non_retryable_failure())
            .map_err(|e| format!("fail_action failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;

        let events = journal
            .events_for_run(run)
            .map_err(|e| format!("events failed: {e}"))?;
        let checkpoint = latest_retry_checkpoint(&events)?;
        let next_seq = next_seq_after(&events)?;
        Ok::<(RunCheckpointV1, EventSeq), String>((checkpoint, next_seq))
    }?;

    // Third attempt: verify actual first-run ticket from run_attempt=1 is rejected
    {
        let journal =
            Arc::new(FjallJournal::open(&path, None).map_err(|e| format!("open journal: {e}"))?);
        let _workflow = retry_action_workflow()?;
        let _contracts = contracts()?;
        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

        runtime
            .recover_retry(run, checkpoint2, next_seq2)
            .map_err(|e| format!("recover_retry failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;

        let current_ticket = pending_ticket(&runtime, run)?;
        assert_eq!(
            current_ticket.run_attempt, 3,
            "third attempt must have run_attempt=3"
        );

        // Stronger frozen public state: capture snapshot before stale rejection.
        let correlation = 0xDEADBEEF;
        let snapshot_before = runtime
            .snapshot_run(run, correlation)
            .map_err(|e| format!("snapshot before stale failed: {e:?}"))?;

        // Capture full event vector BEFORE stale rejection.
        let events_before = journal
            .events_for_run(run)
            .map_err(|e| format!("events before stale failed: {e}"))?;
        let serialized_before = serialize_event_vector(&events_before)?;
        eprintln!("Events before stale rejection: {}", events_before.len());

        // Use retained actual first-run ticket (run_attempt=1) directly as the stale ticket.
        let stale_ticket = first_ticket;

        // Stale completion must be rejected with exact typed error.
        let complete_result =
            runtime.complete_action_with_output(stale_ticket, action_output(SlotValue::I64(42))?);
        match &complete_result {
            Ok(_) => {
                return Err(
                    "stale completion (run_attempt=1 vs current=3) must be rejected".to_string(),
                );
            }
            Err(e) => match e {
                vb_runtime::RuntimeError::StaleAttempt { incoming, current }
                    if *incoming == 1 && *current == 3 => {}
                _ => {
                    return Err(format!(
                        "expected StaleAttempt{{incoming:1,current:3}}, got: {e:?}"
                    ));
                }
            },
        }

        // Immediately after stale completion rejection: journal must be unchanged.
        let events_after_completion_rejection = journal
            .events_for_run(run)
            .map_err(|e| format!("events after completion rejection failed: {e}"))?;
        let serialized_after_completion_rejection =
            serialize_event_vector(&events_after_completion_rejection)?;
        assert_eq!(
            serialized_before, serialized_after_completion_rejection,
            "journal must be byte-identical immediately after stale completion rejection"
        );

        // Verify current pending ticket is still present and unchanged via full Option equality.
        let still_pending_after_stale =
            runtime.lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()));
        assert_eq!(
            still_pending_after_stale,
            Some(current_ticket),
            "current pending ticket must still be present and unchanged after stale completion rejection"
        );

        // Stale failure enqueues without preflight validation (async in shard)
        runtime
            .fail_action(stale_ticket, non_retryable_failure())
            .map_err(|e| format!("stale failure enqueue failed: {e}"))?;

        // Tick must process the queued failure and reject it with exact typed error.
        let tick_result = runtime.tick_all();
        match &tick_result {
            Ok(_) => {
                return Err("tick must reject stale failure".to_string());
            }
            Err(e) => {
                if !matches!(e, vb_runtime::RuntimeError::InvalidActionCompletion) {
                    return Err(format!(
                        "expected InvalidActionCompletion from tick, got: {e:?}"
                    ));
                }
            }
        }

        // Capture full event vector AFTER stale rejection tick.
        let events_after_stale = journal
            .events_for_run(run)
            .map_err(|e| format!("events after stale failed: {e}"))?;
        let serialized_after_stale = serialize_event_vector(&events_after_stale)?;
        eprintln!("Events after stale rejection: {}", events_after_stale.len());
        assert_eq!(
            serialized_before, serialized_after_stale,
            "journal must be byte-identical after stale rejection (no state mutation)"
        );

        // After queued stale failure tick: current pending ticket must still be present.
        let still_pending_after_tick =
            runtime.lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()));
        assert_eq!(
            still_pending_after_tick,
            Some(current_ticket),
            "current pending ticket must still be present and unchanged after queued stale failure tick"
        );

        // Stronger frozen public state: snapshot after stale tick must equal snapshot before.
        let snapshot_after_stale_tick = runtime
            .snapshot_run(run, correlation)
            .map_err(|e| format!("snapshot after stale tick failed: {e:?}"))?;
        assert_eq!(
            snapshot_before, snapshot_after_stale_tick,
            "public inspect snapshot must be identical after stale rejection and stale tick"
        );

        // Current ticket must complete successfully
        let complete_result =
            runtime.complete_action_with_output(current_ticket, action_output(SlotValue::I64(42))?);
        match &complete_result {
            Ok(_) => {}
            Err(e) => {
                return Err(format!(
                    "current ticket (run_attempt=3) must complete successfully: {e:?}"
                ));
            }
        }

        // Tick to process completion
        runtime
            .tick_all()
            .map_err(|e| format!("final tick failed: {e}"))?;

        // Verify run reached RunFinished state with correct result slot.
        let events_after_completion = journal
            .events_for_run(run)
            .map_err(|e| format!("events after completion failed: {e}"))?;
        eprintln!("Events after completion: {}", events_after_completion.len());

        let has_finished_with_correct_result = events_after_completion
            .iter()
            .any(|e| matches!(e, JournalEvent::RunFinished { run: r, result, .. } if *r == run && *result == SlotIdx::new(1)));
        assert!(
            has_finished_with_correct_result,
            "run must have reached RunFinished state with result slot 1"
        );

        // Verify the actual completion output: ticket=current_ticket, output=slot 1, value=I64(42).
        let has_correct_completes_envelope = events_after_completion.iter().any(|e| {
            if let JournalEvent::ActionCompletedEnvelope {
                ticket,
                output,
                value,
                ..
            } = e
            {
                if *ticket == current_ticket && *output == SlotIdx::new(1) {
                    if let Ok(slot_value) = postcard::from_bytes::<SlotValue>(value) {
                        return matches!(slot_value, SlotValue::I64(42));
                    }
                }
            }
            false
        });
        assert!(
            has_correct_completes_envelope,
            "run must have completed with the current ticket on output slot 1 with value I64(42)"
        );

        // Verify current pending ticket has disappeared
        let still_pending =
            runtime.lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()));
        assert!(
            still_pending.is_none(),
            "current pending ticket must disappear after successful completion"
        );
    }

    Ok(())
}

/// Test: future run-attempt ticket is rejected
#[test]
fn future_run_attempt_ticket_rejected() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir failed: {e}"))?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(90002);

    let journal =
        Arc::new(FjallJournal::open(&path, None).map_err(|e| format!("open journal: {e}"))?);
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

    let ticket = pending_ticket(&runtime, run)?;
    assert_eq!(ticket.run_attempt, 1);

    // Craft a future ticket with run_attempt=99
    let future_ticket = ActionTicket {
        run: ticket.run,
        step: ticket.step,
        seq: ticket.seq,
        action: ticket.action,
        run_attempt: 99,
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(
            ticket.run,
            99,
            ticket.seq,
            ticket.action,
        ),
        capacity: ticket.capacity,
    };

    let result =
        runtime.complete_action_with_output(future_ticket, action_output(SlotValue::I64(42))?);
    match &result {
        Ok(_) => {
            return Err(
                "future completion (run_attempt=99 vs current=1) must be rejected".to_string(),
            );
        }
        Err(_) => {}
    }

    Ok(())
}

/// Test: zero run-attempt ticket is rejected
#[test]
fn zero_run_attempt_ticket_rejected() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir failed: {e}"))?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(90003);

    let journal =
        Arc::new(FjallJournal::open(&path, None).map_err(|e| format!("open journal: {e}"))?);
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

    let ticket = pending_ticket(&runtime, run)?;

    // Craft a zero run-attempt ticket
    let zero_ticket = ActionTicket {
        run: ticket.run,
        step: ticket.step,
        seq: ticket.seq,
        action: ticket.action,
        run_attempt: 0,
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(
            ticket.run,
            0,
            ticket.seq,
            ticket.action,
        ),
        capacity: ticket.capacity,
    };

    let result =
        runtime.complete_action_with_output(zero_ticket, action_output(SlotValue::I64(42))?);
    match &result {
        Ok(_) => {
            return Err("zero run-attempt must be rejected".to_string());
        }
        Err(_) => {}
    }

    Ok(())
}

/// Test: cold recovery preserves current generation
#[test]
fn cold_recovery_preserves_run_attempt() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir failed: {e}"))?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(90004);

    // First attempt: fail and checkpoint
    let (checkpoint, next_seq) = {
        let journal =
            Arc::new(FjallJournal::open(&path, None).map_err(|e| format!("open journal: {e}"))?);
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

        let ticket = pending_ticket(&runtime, run)?;
        runtime
            .fail_action(ticket, non_retryable_failure())
            .map_err(|e| format!("fail_action failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;

        let events = journal
            .events_for_run(run)
            .map_err(|e| format!("events failed: {e}"))?;
        let checkpoint = latest_retry_checkpoint(&events)?;
        let next_seq = next_seq_after(&events)?;
        Ok::<(RunCheckpointV1, EventSeq), String>((checkpoint, next_seq))
    }?;

    // Cold reopen and recover: must preserve run_attempt=2
    {
        let journal =
            Arc::new(FjallJournal::open(&path, None).map_err(|e| format!("open journal: {e}"))?);
        let _workflow = retry_action_workflow()?;
        let _contracts = contracts()?;
        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

        runtime
            .recover_retry(run, checkpoint, next_seq)
            .map_err(|e| format!("recover_retry failed: {e}"))?;
        runtime
            .tick_all()
            .map_err(|e| format!("tick failed: {e}"))?;

        let ticket = pending_ticket(&runtime, run)?;
        assert_eq!(
            ticket.run_attempt, 2,
            "cold recovery must preserve run_attempt=2"
        );

        // Old ticket from run_attempt=1 must be rejected
        let old_ticket = ActionTicket {
            run: ticket.run,
            step: ticket.step,
            seq: ticket.seq,
            action: ticket.action,
            run_attempt: 1,
            attempt: 1,
            idempotency_key: vb_core::action::compute_action_idempotency_key(
                ticket.run,
                1,
                ticket.seq,
                ticket.action,
            ),
            capacity: ticket.capacity,
        };

        let result =
            runtime.complete_action_with_output(old_ticket, action_output(SlotValue::I64(42))?);
        match &result {
            Ok(_) => {
                return Err(
                    "old ticket from run_attempt=1 must be rejected after cold recovery"
                        .to_string(),
                );
            }
            Err(_) => {}
        }
    }

    Ok(())
}

/// Test: idempotency key differs across run attempts
#[test]
fn idempotency_key_differs_across_run_attempts() -> Result<(), String> {
    let run = RunId::new(90005);
    let seq = SeqNo::ZERO;
    let action = ActionId::new(1);

    let key1 = vb_core::action::compute_action_idempotency_key(run, 1, seq, action);
    let key2 = vb_core::action::compute_action_idempotency_key(run, 2, seq, action);
    let key3 = vb_core::action::compute_action_idempotency_key(run, 3, seq, action);

    assert_ne!(key1, key2, "keys must differ between run_attempt 1 and 2");
    assert_ne!(key2, key3, "keys must differ between run_attempt 2 and 3");
    assert_ne!(key1, key3, "keys must differ between run_attempt 1 and 3");

    Ok(())
}

/// Test: ticket has valid key after serialization roundtrip
#[test]
fn ticket_key_valid_after_serialization() -> Result<(), String> {
    let run = RunId::new(90006);
    let seq = SeqNo::ZERO;
    let action = ActionId::new(1);
    let run_attempt = 2;

    let ticket = ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq,
        action,
        run_attempt,
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(
            run,
            run_attempt,
            seq,
            action,
        ),
        capacity: 3,
    };

    assert!(
        vb_core::action::action_ticket_has_valid_key(ticket),
        "ticket must have valid key"
    );

    // Serialize and deserialize
    let serialized =
        postcard::to_allocvec(&ticket).map_err(|e| format!("serialize failed: {e}"))?;
    let deserialized: ActionTicket =
        postcard::from_bytes(&serialized).map_err(|e| format!("deserialize failed: {e}"))?;

    assert_eq!(ticket, deserialized, "roundtrip must preserve ticket");
    assert!(
        vb_core::action::action_ticket_has_valid_key(deserialized),
        "deserialized ticket must have valid key"
    );

    Ok(())
}
