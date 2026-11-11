use super::Runtime;
use crate::run_checkpoint::{RunCheckpointV1, decode_checkpoint_payload};
use crate::shard::RunState;
use std::sync::Arc;
use vb_core::action::{
    ActionFailure, ActionFailureCode, ActionOutputReady, ActionTicket, RetryPolicy,
};
use vb_core::{ObjectField, ObjectId, RunId, SlotIdx, SlotValue, StepIdx, SymbolId, Taint};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

#[path = "bound_symbol_duplicate_retry_assertions.rs"]
mod assertions;
#[path = "bound_symbol_duplicate_retry_fixture.rs"]
mod fixture;
use assertions::*;
use fixture::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const RUN: RunId = RunId::new(425_411);
const ACTION: StepIdx = StepIdx::new(1);

struct Evidence {
    checkpoint: RunCheckpointV1,
    next: EventSeq,
    events: Vec<JournalEvent>,
    retained: Vec<u8>,
    before: RunState,
}

#[test]
fn bound_symbol_duplicate_retry_public_producer_preserves_live_tail_cold() -> TestResult {
    let case = Case::new()?;
    let evidence = produce_checkpoint(&case)?;
    let journal = case.open()?;
    assert_eq!(journal.events_for_run_full(RUN)?, evidence.events);
    assert_eq!(case.retained_bytes(&journal)?, evidence.retained);
    let mut runtime = actor(Arc::clone(&journal), 64);
    runtime.recover_retry(RUN, evidence.checkpoint.clone(), evidence.next)?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_recovered(&runtime, &evidence)?;
    finish_retry(&case, &mut runtime, &journal, &evidence)?;
    Ok(())
}

fn produce_checkpoint(case: &Case) -> TestResult<Evidence> {
    let journal = case.open()?;
    let retained = case.retained_bytes(&journal)?;
    let mut runtime = actor(Arc::clone(&journal), 1);
    runtime.activate_owned_submission(RUN, case.source.digest)?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.lookup_pending_action_ticket(RUN, 1), None);
    seed_live_duplicate_tail(&mut runtime)?;
    let before = runtime.inspect_run_state(RUN)?.clone();
    let ticket = dispatch_seeded_action(&mut runtime)?;
    runtime.fail_action(ticket, failure())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    let events = journal.events_for_run_full(RUN)?;
    let (checkpoint, next) = produced_checkpoint(&events)?;
    assert_produced(&checkpoint, &before)?;
    assert_eq!(case.retained_bytes(&journal)?, retained);
    let evidence = Evidence {
        checkpoint,
        next,
        events,
        retained,
        before,
    };
    close_producer(runtime, journal);
    Ok(evidence)
}

fn close_producer(runtime: Runtime, journal: Arc<FjallJournal>) {
    drop(runtime);
    assert_eq!(Arc::strong_count(&journal), 1);
    drop(journal);
}

fn dispatch_seeded_action(runtime: &mut Runtime) -> TestResult<ActionTicket> {
    runtime
        .shards
        .first_mut()
        .ok_or("missing shard")?
        .drive_duplicate_retry_fixture(RUN)?;
    let pending = ticket(runtime)?;
    assert_eq!(pending.run_attempt, 1);
    assert_ticket_input(runtime, pending)?;
    Ok(pending)
}

fn seed_live_duplicate_tail(runtime: &mut Runtime) -> TestResult {
    let shard = runtime.shards.first_mut().ok_or("missing shard")?;
    let state = shard.run_state_get_mut(RUN).ok_or("missing run")?;
    assert_eq!(state.frame.pc(), ACTION);
    assert_eq!(state.store.checkpoint_v1(), initial_runtime_store());
    let tail = state.store.insert_symbol("key")?;
    assert_eq!(tail, SymbolId::new(2));
    let object = state.store.insert_object(Box::from([ObjectField {
        key: tail,
        value: SlotValue::Symbol(SymbolId::new(1)),
        taint: Taint::Clean,
    }]))?;
    assert_eq!(object, ObjectId::new(1));
    state
        .frame
        .write_slot_with_taint(SlotIdx::ZERO, SlotValue::Object(object), Taint::Clean)?;
    seed_collect(state)?;
    assert_literal_state(state)?;
    Ok(())
}

fn seed_collect(state: &mut RunState) -> TestResult {
    let list = state.store.insert_list_with_taint(
        Box::from([SlotValue::Symbol(SymbolId::new(2)), SlotValue::I64(13)]),
        Box::from([Taint::Clean, Taint::Secret]),
    )?;
    assert_eq!(list, vb_core::ListId::new(0));
    state
        .frame
        .write_slot_with_taint(SlotIdx::new(4), SlotValue::List(list), Taint::Secret)?;
    state
        .collect_states
        .upsert(crate::primitives::collect::CollectPaginationState {
            run_id: RUN,
            collector_slot: SlotIdx::new(4),
            source: list,
            current_page: list,
            cursor: 1,
            page_size: 2,
            item_count: 1,
            limit: 3,
            time_limit_ms: Some(50),
            start_millis: 101,
        })
        .map_err(|error| format!("collect setup: {error:?}"))?;
    Ok(())
}

fn failure() -> ActionFailure {
    ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

fn ticket(runtime: &Runtime) -> TestResult<ActionTicket> {
    runtime
        .lookup_pending_action_ticket(RUN, 1)
        .ok_or_else(|| "missing actual ticket".into())
}

fn produced_checkpoint(events: &[JournalEvent]) -> TestResult<(RunCheckpointV1, EventSeq)> {
    let JournalEvent::RetryCheckpoint {
        run,
        attempt,
        step,
        seq,
        payload,
        payload_digest,
        ..
    } = events.last().ok_or("missing producer checkpoint")?
    else {
        return Err("last event is not RetryCheckpoint".into());
    };
    assert_eq!((*run, *attempt, *step), (RUN, 2, ACTION));
    assert_eq!(*payload_digest, *blake3::hash(payload).as_bytes());
    let checkpoint = decode_checkpoint_payload(payload)?;
    assert_failure_precedes(events, &checkpoint, *seq)?;
    let next = seq.get().checked_add(1).ok_or("sequence overflow")?;
    Ok((checkpoint, EventSeq::new(next)))
}

fn assert_failure_precedes(
    events: &[JournalEvent],
    checkpoint: &RunCheckpointV1,
    seq: EventSeq,
) -> TestResult {
    let prior = events
        .iter()
        .rev()
        .nth(1)
        .ok_or("missing preceding failure")?;
    assert!(matches!(prior, JournalEvent::RunFailedEvent { run, attempt: 1, .. } if *run == RUN));
    assert_eq!(prior.seq().get(), checkpoint.prior_seq);
    assert_eq!(checkpoint.prior_seq.checked_add(1), Some(seq.get()));
    Ok(())
}

fn finish_retry(
    case: &Case,
    runtime: &mut Runtime,
    journal: &FjallJournal,
    evidence: &Evidence,
) -> TestResult {
    complete_retry(runtime)?;
    assert_terminal(journal)?;
    assert_eq!(
        runtime.inspect_run_state(RUN).map(|_| ()),
        Err(crate::RuntimeError::RunNotFound)
    );
    assert_eq!(runtime.lookup_pending_action_ticket(RUN, 1), None);
    assert_idle_journal(runtime, journal, &evidence.events)?;
    assert_eq!(case.retained_bytes(journal)?, evidence.retained);
    Ok(())
}

fn complete_retry(runtime: &mut Runtime) -> TestResult {
    let pending = ticket(runtime)?;
    assert_eq!(pending.attempt, 1);
    assert_eq!(pending.run_attempt, 2);
    assert_ticket_input(runtime, pending)?;
    runtime.complete_action_with_output(
        pending,
        ActionOutputReady {
            output_slot: SlotIdx::new(2),
            value: SlotValue::I64(23),
            taint: Taint::Clean,
            encoded_len: 2,
        },
    )?;
    assert_eq!(runtime.tick_all(), Ok(true));
    Ok(())
}

fn assert_idle_journal(
    runtime: &mut Runtime,
    journal: &FjallJournal,
    prefix: &[JournalEvent],
) -> TestResult {
    let after = journal.events_for_run_full(RUN)?;
    assert!(after.iter().take(prefix.len()).eq(prefix.iter()));
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(journal.events_for_run_full(RUN)?, after);
    Ok(())
}
