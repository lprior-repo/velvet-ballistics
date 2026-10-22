//! Runtime authority probe test for vb-79ffw ticket generation and authority.
//!
//! Tests the run-attempt ticket authority mechanism through the real
//! Runtime/Fjall public API. Verifies that stale tickets (wrong run_attempt)
//! are rejected with exact typed errors, and that no state mutation occurs
//! on rejection.

#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{
    action_ticket_has_valid_key, compute_action_idempotency_key, ActionContract, ActionFailure,
    ActionFailureCode, ActionName, ActionOutputReady, ActionTicket, Idempotency, RetryPolicy,
    RetrySafety, SideEffect,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::RuntimeError;
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
    let bytes = postcard::to_allocvec(&canonical)
        .map_err(|e| format!("workflow encode failed: {e}"))?;
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
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| format!("workflow invalid: {e:?}"))
}

fn action_contract() -> Result<ActionContract, String> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("test-action")
            .map_err(|e| format!("action name: {e}"))?,
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
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

fn action_output(value: SlotValue) -> Result<ActionOutputReady, String> {
    let encoded = postcard::to_allocvec(&value)
        .map_err(|e| format!("slot encode failed: {e}"))?;
    Ok(ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint: Taint::Clean,
        encoded_len: u32::try_from(encoded.len())
            .map_err(|_| "encoded length too large for u32".to_string())?,
    })
}

/// Extract the latest pending action ticket from journal events.
fn latest_pending_ticket(events: &[JournalEvent]) -> Result<ActionTicket, String> {
    for event in events.iter().rev() {
        if let JournalEvent::ActionScheduledTicket { ticket, .. } = event {
            return Ok(ticket.clone());
        }
    }
    Err("no ActionScheduledTicket event found".to_string())
}

fn latest_retry_checkpoint(events: &[JournalEvent]) -> Result<RunCheckpointV1, String> {
    let payload = events
        .iter()
        .rev()
        .find_map(|e| match e {
            JournalEvent::RetryCheckpoint { payload, .. } => Some(payload.clone()),
            _ => None,
        })
        .ok_or("no retry checkpoint event found".to_string())?;
    postcard::from_bytes(&payload)
        .map_err(|e| format!("checkpoint decode failed: {e}"))
}

fn next_seq_after(events: &[JournalEvent]) -> Result<EventSeq, String> {
    let max_seq = events.iter().map(|e| e.seq().get()).max()
        .ok_or("no events to derive sequence from".to_string())?;
    Ok(EventSeq::new(max_seq + 1))
}

/// Serialize events for exact comparison.
fn serialize_events(events: &[JournalEvent]) -> Result<Vec<Vec<u8>>, String> {
    let mut result = Vec::with_capacity(events.len());
    for event in events {
        let bytes = postcard::to_allocvec(event)
            .map_err(|e| format!("event serialize failed: {e}"))?;
        result.push(bytes);
    }
    Ok(result)
}

/// Test: stale completion is rejected after 2+ retries with no state mutation.
#[test]
fn stale_completion_rejected_after_two_retries() -> Result<(), String> {
    let dir = tempfile::tempdir()
        .map_err(|e| format!("tempdir failed: {e}"))?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(90001);

    // First attempt: fail and checkpoint
    let (checkpoint1, next_seq1) = {
        let journal = Arc::new(FjallJournal::open(&path, None)
            .map_err(|e| format!("open journal: {e}"))?);
        let workflow = retry_action_workflow()?;
        let contracts = contracts()?;
        vb_storage::submit_artifact_with_contracts(
            &journal, &workflow, RuntimePolicy::Strict, &contracts,
        ).map_err(|e| format!("artifact submit failed: {e}"))?;

        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
        runtime.submit_direct_with_inputs_grants_and_contracts(
            run, workflow, Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            CapabilitySet::empty(), contracts,
        ).map_err(|e| format!("submit failed: {e}"))?;
        runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

        let events = journal.events_for_run(run)
            .map_err(|e| format!("events failed: {e}"))?;
        let ticket = latest_pending_ticket(&events)?;
        if ticket.run_attempt != 1 {
            return Err(format!("first attempt must have run_attempt=1, got {}", ticket.run_attempt));
        }

        runtime.fail_action(ticket, non_retryable_failure())
            .map_err(|e| format!("fail_action failed: {e}"))?;
        runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

        let events = journal.events_for_run(run).map_err(|e| format!("events failed: {e}"))?;
        let checkpoint = latest_retry_checkpoint(&events)?;
        let next_seq = next_seq_after(&events)?;
        Ok::<(RunCheckpointV1, EventSeq), String>((checkpoint, next_seq))
    }?;

    // Second attempt: fail and checkpoint again
    let (checkpoint2, next_seq2) = {
        let journal = Arc::new(FjallJournal::open(&path, None)
            .map_err(|e| format!("open journal: {e}"))?);
        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

        runtime.recover_retry(run, checkpoint1, next_seq1)
            .map_err(|e| format!("recover_retry failed: {e}"))?;
        runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

        let events = journal.events_for_run(run)
            .map_err(|e| format!("events failed: {e}"))?;
        let ticket = latest_pending_ticket(&events)?;
        if ticket.run_attempt != 2 {
            return Err(format!("second attempt must have run_attempt=2, got {}", ticket.run_attempt));
        }

        runtime.fail_action(ticket, non_retryable_failure())
            .map_err(|e| format!("fail_action failed: {e}"))?;
        runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

        let events = journal.events_for_run(run).map_err(|e| format!("events failed: {e}"))?;
        let checkpoint = latest_retry_checkpoint(&events)?;
        let next_seq = next_seq_after(&events)?;
        Ok::<(RunCheckpointV1, EventSeq), String>((checkpoint, next_seq))
    }?;

    // Third attempt: verify old tickets from run_attempt=1 are rejected
    {
        let journal = Arc::new(FjallJournal::open(&path, None)
            .map_err(|e| format!("open journal: {e}"))?);
        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

        runtime.recover_retry(run, checkpoint2, next_seq2)
            .map_err(|e| format!("recover_retry failed: {e}"))?;
        runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

        let events = journal.events_for_run(run)
            .map_err(|e| format!("events failed: {e}"))?;
        let current_ticket = latest_pending_ticket(&events)?;
        if current_ticket.run_attempt != 3 {
            return Err(format!("third attempt must have run_attempt=3, got {}", current_ticket.run_attempt));
        }

        // Capture full event vector BEFORE stale rejection
        let events_before = journal.events_for_run(run)
            .map_err(|e| format!("events before stale failed: {e}"))?;
        let serialized_before = serialize_events(&events_before)?;
        eprintln!("Events before stale rejection: {}", events_before.len());

        // Craft a stale ticket with attempt=1 (older than current attempt=3)
        let stale_ticket = ActionTicket {
            run: current_ticket.run,
            step: current_ticket.step,
            seq: current_ticket.seq,
            action: current_ticket.action,
            run_attempt: current_ticket.run_attempt,
            attempt: 1,
            idempotency_key: compute_action_idempotency_key(
                current_ticket.run, current_ticket.run_attempt, current_ticket.seq, current_ticket.action),
            capacity: current_ticket.capacity,
        };

        // Stale completion must be rejected with exact typed error
        let result = runtime.complete_action_with_output(
            stale_ticket, action_output(SlotValue::I64(42))?);
        match result {
            Err(RuntimeError::StaleAttempt { incoming, current }) => {
                if incoming != 1 || current != 3 {
                    return Err(format!(
                        "stale attempt error has wrong fields: incoming={} current={} (expected 1/3)",
                        incoming, current));
                }
                eprintln!("Stale completion rejected with correct StaleAttempt error");
            }
            Err(e) => {
                return Err(format!("stale completion rejected with wrong error type: {}", e));
            }
            Ok(()) => {
                return Err("stale completion (run_attempt=1 vs current=3) must be rejected".to_string());
            }
        }

        // Verify no state mutation after stale completion rejection
        let events_after_completion = journal.events_for_run(run)
            .map_err(|e| format!("events after stale completion failed: {e}"))?;
        let serialized_after_completion = serialize_events(&events_after_completion)?;
        if serialized_before != serialized_after_completion {
            return Err("state mutated after stale completion rejection".to_string());
        }
        eprintln!("No state mutation after stale completion rejection");

        // Stale failure enqueues without preflight validation (async in shard)
        let result = runtime.fail_action(stale_ticket, non_retryable_failure());
        match result {
            Ok(()) => eprintln!("Stale failure enqueued"),
            Err(e) => {
                return Err(format!("stale failure enqueue failed: {}", e));
            }
        }

        // Tick must process the queued failure and reject it with exact error
        let tick_result = runtime.tick_all();
        match tick_result {
            Err(RuntimeError::StaleAttempt { incoming, current }) => {
                if incoming != 1 || current != 3 {
                    return Err(format!(
                        "stale failure tick error has wrong fields: incoming={} current={} (expected 1/3)",
                        incoming, current));
                }
                eprintln!("Tick rejected stale failure with correct StaleAttempt error");
            }
            Err(e) => {
                return Err(format!("tick rejected stale failure with wrong error type: {}", e));
            }
            Ok(_) => {
                return Err("tick must reject queued stale failure".to_string());
            }
        }

        // Verify no state mutation after stale failure tick
        let events_after_stale_tick = journal.events_for_run(run)
            .map_err(|e| format!("events after stale tick failed: {e}"))?;
        let serialized_after_stale_tick = serialize_events(&events_after_stale_tick)?;
        if serialized_before != serialized_after_stale_tick {
            return Err("state mutated after stale failure tick".to_string());
        }
        eprintln!("No state mutation after stale failure tick");

        // Current ticket must complete successfully
        let result = runtime.complete_action_with_output(
            current_ticket, action_output(SlotValue::I64(42))?);
        match result {
            Ok(()) => eprintln!("Current ticket completed"),
            Err(e) => {
                return Err(format!("current ticket completion failed: {}", e));
            }
        }

        // Tick to process completion
        runtime.tick_all().map_err(|e| format!("final tick failed: {e}"))?;

        // Verify run reached RunFinished state
        let events_after = journal.events_for_run(run)
            .map_err(|e| format!("events after completion failed: {e}"))?;
        let has_finished = events_after.iter().any(|e| {
            matches!(e, JournalEvent::RunFinished { .. })
        });
        if !has_finished {
            return Err("run must have reached RunFinished state after current ticket completion".to_string());
        }
        eprintln!("Run reached RunFinished state");
    }

    Ok(())
}

/// Test: future run-attempt ticket is rejected with exact error.
#[test]
fn future_run_attempt_ticket_rejected() -> Result<(), String> {
    let dir = tempfile::tempdir()
        .map_err(|e| format!("tempdir failed: {e}"))?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(90002);

    let journal = Arc::new(FjallJournal::open(&path, None)
        .map_err(|e| format!("open journal: {e}"))?);
    let workflow = retry_action_workflow()?;
    let contracts = contracts()?;
    vb_storage::submit_artifact_with_contracts(
        &journal, &workflow, RuntimePolicy::Strict, &contracts,
    ).map_err(|e| format!("artifact submit failed: {e}"))?;

    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run, workflow, Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
        CapabilitySet::empty(), contracts,
    ).map_err(|e| format!("submit failed: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

    let events = journal.events_for_run(run)
        .map_err(|e| format!("events failed: {e}"))?;
    let ticket = latest_pending_ticket(&events)?;
    if ticket.run_attempt != 1 {
        return Err(format!("expected run_attempt=1, got {}", ticket.run_attempt));
    }

    // Craft a future ticket with run_attempt=99
    let future_ticket = ActionTicket {
        run: ticket.run,
        step: ticket.step,
        seq: ticket.seq,
        action: ticket.action,
        run_attempt: 99,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(
            ticket.run, 99, ticket.seq, ticket.action),
        capacity: ticket.capacity,
    };

    let result = runtime.complete_action_with_output(
        future_ticket, action_output(SlotValue::I64(42))?);
    match result {
        Err(RuntimeError::StaleAttempt { incoming, current }) => {
            if incoming != 99 || current != 1 {
                return Err(format!(
                    "future attempt error has wrong fields: incoming={} current={} (expected 99/1)",
                    incoming, current));
            }
            eprintln!("Future ticket rejected with correct StaleAttempt error");
        }
        Err(e) => {
            return Err(format!("future ticket rejected with wrong error type: {}", e));
        }
        Ok(()) => {
            return Err("future completion (run_attempt=99 vs current=1) must be rejected".to_string());
        }
    }

    Ok(())
}

/// Test: zero run-attempt ticket is rejected with exact error.
#[test]
fn zero_run_attempt_ticket_rejected() -> Result<(), String> {
    let dir = tempfile::tempdir()
        .map_err(|e| format!("tempdir failed: {e}"))?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(90003);

    let journal = Arc::new(FjallJournal::open(&path, None)
        .map_err(|e| format!("open journal: {e}"))?);
    let workflow = retry_action_workflow()?;
    let contracts = contracts()?;
    vb_storage::submit_artifact_with_contracts(
        &journal, &workflow, RuntimePolicy::Strict, &contracts,
    ).map_err(|e| format!("artifact submit failed: {e}"))?;

    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run, workflow, Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
        CapabilitySet::empty(), contracts,
    ).map_err(|e| format!("submit failed: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

    let events = journal.events_for_run(run)
        .map_err(|e| format!("events failed: {e}"))?;
    let ticket = latest_pending_ticket(&events)?;

    // Craft a zero run-attempt ticket
    let zero_ticket = ActionTicket {
        run: ticket.run,
        step: ticket.step,
        seq: ticket.seq,
        action: ticket.action,
        run_attempt: 0,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(
            ticket.run, 0, ticket.seq, ticket.action),
        capacity: ticket.capacity,
    };

    let result = runtime.complete_action_with_output(
        zero_ticket, action_output(SlotValue::I64(42))?);
    match result {
        Err(RuntimeError::StaleAttempt { incoming, current }) => {
            if incoming != 0 || current != 1 {
                return Err(format!(
                    "zero attempt error has wrong fields: incoming={} current={} (expected 0/1)",
                    incoming, current));
            }
            eprintln!("Zero ticket rejected with correct StaleAttempt error");
        }
        Err(e) => {
            return Err(format!("zero ticket rejected with wrong error type: {}", e));
        }
        Ok(()) => {
            return Err("zero run-attempt must be rejected".to_string());
        }
    }

    Ok(())
}

/// Test: cold recovery preserves current generation.
#[test]
fn cold_recovery_preserves_run_attempt() -> Result<(), String> {
    let dir = tempfile::tempdir()
        .map_err(|e| format!("tempdir failed: {e}"))?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(90004);

    // First attempt: fail and checkpoint
    let (checkpoint, next_seq) = {
        let journal = Arc::new(FjallJournal::open(&path, None)
            .map_err(|e| format!("open journal: {e}"))?);
        let workflow = retry_action_workflow()?;
        let contracts = contracts()?;
        vb_storage::submit_artifact_with_contracts(
            &journal, &workflow, RuntimePolicy::Strict, &contracts,
        ).map_err(|e| format!("artifact submit failed: {e}"))?;

        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
        runtime.submit_direct_with_inputs_grants_and_contracts(
            run, workflow, Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            CapabilitySet::empty(), contracts,
        ).map_err(|e| format!("submit failed: {e}"))?;
        runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

        let events = journal.events_for_run(run)
            .map_err(|e| format!("events failed: {e}"))?;
        let ticket = latest_pending_ticket(&events)?;
        runtime.fail_action(ticket, non_retryable_failure())
            .map_err(|e| format!("fail_action failed: {e}"))?;
        runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

        let events = journal.events_for_run(run).map_err(|e| format!("events failed: {e}"))?;
        let checkpoint = latest_retry_checkpoint(&events)?;
        let next_seq = next_seq_after(&events)?;
        Ok::<(RunCheckpointV1, EventSeq), String>((checkpoint, next_seq))
    }?;

    // Cold reopen and recover: must preserve run_attempt=2
    {
        let journal = Arc::new(FjallJournal::open(&path, None)
            .map_err(|e| format!("open journal: {e}"))?);
        let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
        let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);

        runtime.recover_retry(run, checkpoint, next_seq)
            .map_err(|e| format!("recover_retry failed: {e}"))?;
        runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

        let events = journal.events_for_run(run)
            .map_err(|e| format!("events failed: {e}"))?;
        let ticket = latest_pending_ticket(&events)?;
        if ticket.run_attempt != 2 {
            return Err(format!("cold recovery must preserve run_attempt=2, got {}", ticket.run_attempt));
        }

        // Old ticket from run_attempt=1 must be rejected
        let old_ticket = ActionTicket {
            run: ticket.run,
            step: ticket.step,
            seq: ticket.seq,
            action: ticket.action,
            run_attempt: 1,
            attempt: 1,
            idempotency_key: compute_action_idempotency_key(
                ticket.run, 1, ticket.seq, ticket.action),
            capacity: ticket.capacity,
        };

        let result = runtime.complete_action_with_output(
            old_ticket, action_output(SlotValue::I64(42))?);
        match result {
            Err(RuntimeError::StaleAttempt { incoming, current }) => {
                if incoming != 1 || current != 2 {
                    return Err(format!(
                        "old ticket after recovery error has wrong fields: incoming={} current={} (expected 1/2)",
                        incoming, current));
                }
                eprintln!("Old ticket rejected after cold recovery with correct error");
            }
            Err(e) => {
                return Err(format!("old ticket rejected with wrong error type: {}", e));
            }
            Ok(()) => {
                return Err("old ticket from run_attempt=1 must be rejected after cold recovery".to_string());
            }
        }
    }

    Ok(())
}

/// Test: idempotency key differs across run attempts.
#[test]
fn idempotency_key_differs_across_run_attempts() -> Result<(), String> {
    let run = RunId::new(90005);
    let seq = SeqNo::ZERO;
    let action = ActionId::new(1);

    let key1 = compute_action_idempotency_key(run, 1, seq, action);
    let key2 = compute_action_idempotency_key(run, 2, seq, action);
    let key3 = compute_action_idempotency_key(run, 3, seq, action);

    if key1 == key2 {
        return Err("keys must differ between run_attempt 1 and 2".to_string());
    }
    if key2 == key3 {
        return Err("keys must differ between run_attempt 2 and 3".to_string());
    }
    if key1 == key3 {
        return Err("keys must differ between run_attempt 1 and 3".to_string());
    }

    Ok(())
}

/// Test: ticket has valid key after serialization roundtrip.
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
        idempotency_key: compute_action_idempotency_key(
            run, run_attempt, seq, action),
        capacity: 3,
    };

    if !action_ticket_has_valid_key(ticket) {
        return Err("ticket must have valid key".to_string());
    }

    // Serialize and deserialize
    let serialized = postcard::to_allocvec(&ticket)
        .map_err(|e| format!("serialize failed: {e}"))?;
    let deserialized: ActionTicket = postcard::from_bytes(&serialized)
        .map_err(|e| format!("deserialize failed: {e}"))?;

    if ticket != deserialized {
        return Err("roundtrip must preserve ticket".to_string());
    }
    if !action_ticket_has_valid_key(deserialized) {
        return Err("deserialized ticket must have valid key".to_string());
    }

    Ok(())
}