#![forbid(unsafe_code)]
//! vb-r4fm9 regression: ticket-bearing journal events must report run-level attempt.
//!
//! After a durable run failure and explicit retry, the journal contains events from
//! run attempt 2. All events must report attempt=2 for correct latest-attempt replay
//! filtering. Before the fix, ticket-bearing events (ActionScheduledTicket,
//! ActionCompletedEnvelope, ActionAbandoned) reported the per-action attempt (always 1),
//! causing them to be filtered out as stale during cold replay.
//!
//! This test demonstrates the defect: it produces a durable retry scenario and verifies
//! that journal events for run attempt 2 correctly report attempt=2 via the accessor.

use std::sync::Arc;

use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket,
    Idempotency, RetryPolicy, RetrySafety, SideEffect,
};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::run_checkpoint::RunCheckpointV1;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

fn shard_count() -> Result<std::num::NonZeroUsize, String> {
    std::num::NonZeroUsize::new(1).ok_or_else(|| "shard count must be non-zero".to_string())
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
        name: Box::from("vb-r4fm9-retry"),
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
        .map_err(|e| format!("retry workflow invalid: {e:?}"))
}

fn action_contract() -> ActionContract {
    ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("vb-r4fm9-action").expect("valid action name"),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([]),
    }
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

fn action_output(value: SlotValue) -> ActionOutputReady {
    let encoded = postcard::to_allocvec(&value).expect("slot encoding");
    let encoded_len = u32::try_from(encoded.len()).expect("slot length fits u32");
    ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint: Taint::Clean,
        encoded_len,
    }
}

fn latest_retry_checkpoint(events: &[JournalEvent]) -> RunCheckpointV1 {
    let JournalEvent::RetryCheckpoint { payload, .. } = events
        .iter()
        .rev()
        .find(|e| matches!(e, JournalEvent::RetryCheckpoint { .. }))
        .expect("no durable retry checkpoint in journal")
    else {
        panic!("no durable retry checkpoint in journal");
    };
    postcard::from_bytes::<RunCheckpointV1>(payload).expect("checkpoint decode")
}

fn next_seq_after(events: &[JournalEvent]) -> EventSeq {
    let last = events
        .last()
        .map(JournalEvent::seq)
        .unwrap_or(EventSeq::ZERO)
        .get();
    EventSeq::new(last + 1)
}

/// Produces a durable failed run attempt 1 and captures the old ticket.
fn produce_failed_attempt(path: &std::path::Path, run: RunId) -> Result<(RunCheckpointV1, EventSeq, ActionTicket), String> {
    let journal = Arc::new(FjallJournal::open(path, None).map_err(|e| e.to_string())?);
    let workflow = retry_action_workflow()?;
    let contracts = Box::from([action_contract()]);
    vb_storage::submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
        .map_err(|e| format!("artifact submit failed: {e}"))?;

    let shared = vb_runtime::journal::StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow,
            Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            vb_core::capability::CapabilitySet::empty(),
            contracts,
        )
        .map_err(|e| format!("submit failed: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

    let old_ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or_else(|| "run has no pending action ticket")?;
    runtime
        .fail_action(old_ticket, non_retryable_failure())
        .map_err(|e| format!("fail_action failed: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;
    let checkpoint = latest_retry_checkpoint(&events);
    let next_seq = next_seq_after(&events);
    Ok((checkpoint, next_seq, old_ticket))
}

/// Cold-reopens and retries the run, driving it to the new pending action at run_attempt=2.
fn recover_and_schedule(
    path: &std::path::Path,
    run: RunId,
    checkpoint: RunCheckpointV1,
    next_seq: EventSeq,
) -> Result<Runtime, String> {
    let journal = Arc::new(FjallJournal::open(path, None).map_err(|e| e.to_string())?);
    let shared = vb_runtime::journal::StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
    runtime
        .recover_retry(run, checkpoint, next_seq)
        .map_err(|e| format!("recover_retry failed: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;
    Ok(runtime)
}

/// RED test: ticket-bearing events from run attempt 2 must report attempt=2.
/// Before the fix, they report attempt=1 (per-action), which is wrong.
#[test]
fn ticket_events_report_run_attempt_on_retry() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(42);

    let (checkpoint, next_seq, old_ticket) = produce_failed_attempt(&path, run)?;
    assert_eq!(old_ticket.run_attempt, 1, "old ticket must be run_attempt=1");

    recover_and_schedule(&path, run, checkpoint, next_seq)?;

    // After retry, read the full journal and inspect all events for run attempt 2.
    let journal = FjallJournal::open(&path, None).map_err(|e| e.to_string())?;
    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;

    // Find ticket-bearing events from run attempt 2.
    let ticket_events: Vec<&JournalEvent> = events
        .iter()
        .filter(|e| matches!(
            e,
            JournalEvent::ActionScheduledTicket { .. }
                | JournalEvent::ActionCompletedEnvelope { .. }
                | JournalEvent::ActionAbandoned { .. }
        ))
        .collect();

    assert!(
        !ticket_events.is_empty(),
        "expected ticket-bearing events from run attempt 2"
    );

    // The second ticket-bearing event should be from run attempt 2.
    let attempt2_ticket_event = ticket_events
        .iter()
        .find(|e| match e {
            JournalEvent::ActionScheduledTicket { ticket, .. }
            | JournalEvent::ActionCompletedEnvelope { ticket, .. }
            | JournalEvent::ActionAbandoned { ticket, .. } => ticket.run_attempt == 2,
            _ => false,
        })
        .expect("expected ActionScheduledTicket with run_attempt=2");

    // This is the assertion that fails before the fix:
    // attempt() returns ticket.attempt (per-action, 1) instead of ticket.run_attempt (run-level, 2).
    assert_eq!(
        attempt2_ticket_event.attempt(),
        Some(2),
        "ticket-bearing event from run attempt 2 must report attempt=2 via accessor"
    );

    Ok(())
}

/// Regression: cold replay of run attempt 2 must include ticket-bearing events.
/// Before the fix, these events would be filtered out as "stale" (attempt=1 < max=2).
#[test]
fn cold_replay_preserves_run_attempt_2_ticket_events() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(43);

    // Produce failed attempt 1
    let (checkpoint, next_seq, old_ticket) = produce_failed_attempt(&path, run)?;
    assert_eq!(old_ticket.run_attempt, 1);

    {
        let mut runtime = recover_and_schedule(&path, run, checkpoint, next_seq)?;

        // Verify the new ticket has run_attempt=2, per-action=1
        let new_ticket = runtime
            .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
            .ok_or_else(|| "run attempt 2 has no pending ticket")?;
        assert_eq!(new_ticket.run_attempt, 2, "new ticket must be run_attempt=2");
        assert_eq!(new_ticket.attempt, 1, "per-action attempt must be 1");

        // Complete the action on run attempt 2
        runtime
            .complete_action_with_output(new_ticket, action_output(SlotValue::I64(42)))
            .map_err(|e| format!("complete_action failed: {e}"))?;
        runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;
    }

    // Now cold-replay the journal from scratch
    let journal = FjallJournal::open(&path, None).map_err(|e| e.to_string())?;
    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;

    // Verify that ticket-bearing events from run attempt 2 are present
    let attempt2_events: Vec<&JournalEvent> = events
        .iter()
        .filter(|e| e.attempt() == Some(2))
        .collect();

    assert!(
        !attempt2_events.is_empty(),
        "run attempt 2 must have events with attempt=2"
    );

    // Verify ActionScheduledTicket for run attempt 2 is present
    let has_scheduled_ticket = events.iter().any(|e| matches!(
        e,
        JournalEvent::ActionScheduledTicket { ticket, .. } if ticket.run_attempt == 2
    ));
    assert!(has_scheduled_ticket, "ActionScheduledTicket for run attempt 2 must be present");

    // Verify ActionCompletedEnvelope for run attempt 2 is present
    let has_completed = events.iter().any(|e| matches!(
        e,
        JournalEvent::ActionCompletedEnvelope { ticket, .. } if ticket.run_attempt == 2
    ));
    assert!(has_completed, "ActionCompletedEnvelope for run attempt 2 must be present");

    Ok(())
}

/// Regression: completed retry must replay as completed, not lose the result.
#[test]
fn completed_retry_replays_as_completed() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(44);

    // Produce failed attempt 1
    let (checkpoint, next_seq, old_ticket) = produce_failed_attempt(&path, run)?;
    assert_eq!(old_ticket.run_attempt, 1);

    {
        let mut runtime = recover_and_schedule(&path, run, checkpoint, next_seq)?;

        // Complete the action on run attempt 2
        let new_ticket = runtime
            .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
            .ok_or_else(|| "run attempt 2 has no pending ticket")?;
        runtime
            .complete_action_with_output(new_ticket, action_output(SlotValue::I64(99)))
            .map_err(|e| format!("complete_action failed: {e}"))?;
        runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;
    }

    // Cold-replay from scratch
    let journal = FjallJournal::open(&path, None).map_err(|e| e.to_string())?;
    let events = journal.events_for_run(run).map_err(|e| e.to_string())?;

    // The run should replay as completed (not failed or pending)
    let has_finished = events.iter().any(|e| matches!(e, JournalEvent::RunFinished { .. }));
    assert!(has_finished, "replayed run must be finished");

    // The ActionCompletedEnvelope must be present (not filtered as stale)
    let has_completed_envelope = events.iter().any(|e| matches!(
        e,
        JournalEvent::ActionCompletedEnvelope { .. }
    ));
    assert!(has_completed_envelope, "ActionCompletedEnvelope must survive replay");

    Ok(())
}