#![forbid(unsafe_code)]
use std::num::NonZeroUsize;
use vb_core::{RunId, RuntimePolicy};
use vb_runtime::{
    RuntimeError,
    journal::{RuntimeJournalEvent, SharedRuntimeJournal},
    runtime::Runtime,
    shard::ShardConfig,
};
use vb_storage::{FjallJournal, JournalEvent};
#[path = "runtime_immediate_retry_atomic_producer/budgets.rs"]
mod budgets;
#[path = "runtime_immediate_retry_atomic_producer/cut.rs"]
mod cut;
#[path = "runtime_immediate_retry_atomic_producer/fixture.rs"]
mod fixture;
#[path = "runtime_immediate_retry_atomic_producer/low_budget.rs"]
mod low_budget;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const RUN: RunId = RunId::new(92429);
fn runtime(journal: SharedRuntimeJournal) -> TestResult<Runtime> {
    runtime_with_budget(journal, 16)
}

fn runtime_with_budget(
    journal: SharedRuntimeJournal,
    step_budget_per_tick: u64,
) -> TestResult<Runtime> {
    Ok(Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick,
            ..ShardConfig::default()
        },
        journal,
    )?)
}

fn batch_shape(
    events: &[RuntimeJournalEvent],
    old: vb_core::ActionTicket,
) -> TestResult<vb_core::ActionTicket> {
    let [
        RuntimeJournalEvent::ActionFailed {
            run,
            step,
            action,
            attempt,
        },
        RuntimeJournalEvent::StepStarted {
            run: started_run,
            step: started_step,
            attempt: run_attempt,
        },
        RuntimeJournalEvent::ActionScheduledTicket { ticket, .. },
    ] = events
    else {
        return Err(
            format!("expected one atomic failure/start/schedule batch, got {events:?}").into(),
        );
    };
    assert_eq!(
        (*run, *step, *action, *attempt),
        (old.run, old.step, old.action, old.attempt)
    );
    assert_eq!(
        (*started_run, *started_step, *run_attempt),
        (old.run, old.step, old.run_attempt)
    );
    assert_eq!(
        (
            ticket.run,
            ticket.step,
            ticket.action,
            ticket.seq,
            ticket.idempotency_key,
            ticket.capacity,
            ticket.run_attempt
        ),
        (
            old.run,
            old.step,
            old.action,
            old.seq,
            old.idempotency_key,
            old.capacity,
            old.run_attempt
        )
    );
    assert_eq!(Some(ticket.attempt), old.attempt.checked_add(1));
    assert!(ticket.fencing_token > old.fencing_token);
    Ok(*ticket)
}

#[test]
fn retry_precommit_failure_preserves_all_cold_rows_and_old_ticket() -> TestResult {
    let case = fixture::Case::new(false, false)?;
    let before = fixture::rows(case.directory.path())?;
    let (journal, fault, mut live) = case.open(cut::Mode::Reject)?;
    let state = live.inspect_run_state(RUN)?.clone();
    let pending = live.pending_boundary_snapshot(8);
    let counters = live.counters_snapshot();
    let quota = live.usage_snapshot();
    let trace = live.list_events(RUN)?;
    let events = journal.events_for_run_full(RUN)?;
    let receipt = case.receipt(&journal)?;
    live.fail_action(case.ticket, fixture::failure())?;
    assert_eq!(live.tick_all(), Err(RuntimeError::JournalPoisoned));
    batch_shape(&fault.captured()?.1, case.ticket)?;
    assert_eq!(live.inspect_run_state(RUN)?, &state);
    assert_eq!(live.lookup_pending_action_ticket(RUN, 0), Some(case.ticket));
    assert_eq!(live.pending_boundary_snapshot(8), pending);
    assert_eq!(live.counters_snapshot(), counters);
    assert_eq!(live.usage_snapshot(), quota);
    assert_eq!(live.list_events(RUN)?, trace);
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    assert_eq!(case.receipt(&journal)?, receipt);
    drop(live);
    drop(fault);
    drop(journal);
    assert_eq!(fixture::rows(case.directory.path())?, before);
    Ok(())
}

#[test]
fn rejected_retry_still_completes_the_original_live_fenced_ticket() -> TestResult {
    let case = fixture::Case::new(true, true)?;
    let (journal, fault, mut live) = case.open(cut::Mode::Reject)?;
    live.fail_action(case.ticket, fixture::failure())?;
    assert_eq!(live.tick_all(), Err(RuntimeError::JournalPoisoned));
    batch_shape(&fault.captured()?.1, case.ticket)?;
    assert_eq!(
        live.complete_action_with_output(case.ticket, case.output()?),
        Ok(())
    );
    live.tick_all()?;
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(events.iter().filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { ticket, .. } if *ticket == case.ticket)).count(), 1);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionFailedEvent { .. }))
            .count(),
        0
    );
    Ok(())
}

#[test]
fn committed_retry_publishes_the_real_three_record_batch_cold() -> TestResult {
    let case = fixture::Case::new(true, false)?;
    let (journal, fault, mut live) = case.open(cut::Mode::Observe)?;
    let before = journal.events_for_run_full(RUN)?;
    let receipt = case.receipt(&journal)?;
    live.fail_action(case.ticket, fixture::failure())?;
    live.tick_all()?;
    let (seq, batch) = fault.captured()?;
    let next = batch_shape(&batch, case.ticket)?;
    assert_eq!(live.lookup_pending_action_ticket(RUN, 0), Some(next));
    let expected = journal.events_for_run_full(RUN)?;
    let old_bindings = before
        .iter()
        .find_map(|event| match event {
            JournalEvent::ActionScheduledTicket {
                ticket,
                input,
                output,
                action_abi_digest,
                ..
            } if *ticket == case.ticket => Some((*input, *output, *action_abi_digest)),
            _ => None,
        })
        .ok_or("old scheduled bindings absent")?;
    let next_bindings = expected
        .iter()
        .find_map(|event| match event {
            JournalEvent::ActionScheduledTicket {
                ticket,
                input,
                output,
                action_abi_digest,
                ..
            } if *ticket == next => Some((*input, *output, *action_abi_digest)),
            _ => None,
        })
        .ok_or("new scheduled bindings absent")?;
    assert_eq!(next_bindings, old_bindings);
    assert_eq!(expected.get(..before.len()), Some(before.as_slice()));
    assert_eq!(
        expected.len(),
        before.len().checked_add(3).ok_or("length overflow")?
    );
    let encoded: Vec<_> = batch
        .into_iter()
        .enumerate()
        .map(|(i, event)| -> TestResult<_> {
            let at = vb_storage::EventSeq::new(
                seq.get()
                    .checked_add(u64::try_from(i)?)
                    .ok_or("sequence overflow")?,
            );
            Ok((
                at,
                vb_runtime::journal::RuntimeJournal::convert_to_storage_event(
                    fault.as_ref(),
                    event,
                    at,
                )?,
            ))
        })
        .collect::<TestResult<_>>()?;
    drop(live);
    drop(fault);
    drop(journal);
    let cold = FjallJournal::open(case.directory.path(), None)?;
    assert_eq!(cold.events_for_run_full(RUN)?, expected);
    assert_eq!(case.receipt(&cold)?, receipt);
    for (seq, event) in encoded {
        assert!(cold.get_event_bytes(RUN, seq)?.is_some());
        assert!(expected.contains(&event));
    }
    Ok(())
}

#[test]
fn third_record_duplicate_preserves_old_index_and_no_retry_prefix() -> TestResult {
    let case = fixture::Case::new(false, true)?;
    let (journal, fault, mut live) = case.open(cut::Mode::Conflict)?;
    let state = live.inspect_run_state(RUN)?.clone();
    let pending = live.pending_boundary_snapshot(8);
    let counters = live.counters_snapshot();
    let quota = live.usage_snapshot();
    let trace = live.list_events(RUN)?;
    let prefix = journal.events_for_run_full(RUN)?;
    let receipt = case.receipt(&journal)?;
    drop(journal);
    live.fail_action(case.ticket, fixture::failure())?;
    let result = live.tick_all();
    let (seq, batch) = fault.captured()?;
    batch_shape(&batch, case.ticket)?;
    let third = vb_storage::EventSeq::new(seq.get().checked_add(2).ok_or("sequence overflow")?);
    assert!(
        matches!(&result, Err(RuntimeError::StorageJournalAppend { source }) if matches!(source.as_ref(), vb_storage::JournalError::DuplicateEvent { run, seq } if *run == RUN && *seq == third)),
        "{result:?}"
    );
    assert_eq!(live.inspect_run_state(RUN)?, &state);
    assert_eq!(live.lookup_pending_action_ticket(RUN, 0), Some(case.ticket));
    assert_eq!(live.pending_boundary_snapshot(8), pending);
    assert_eq!(live.counters_snapshot(), counters);
    assert_eq!(live.usage_snapshot(), quota);
    assert_eq!(live.list_events(RUN)?, trace);
    let journal = fault.journal();
    assert_eq!(case.receipt(&journal)?, receipt);
    assert_eq!(journal.get_event_bytes(RUN, seq)?, None);
    assert_eq!(
        journal.get_event_bytes(
            RUN,
            vb_storage::EventSeq::new(seq.get().checked_add(1).ok_or("sequence overflow")?)
        )?,
        None
    );
    for event in prefix {
        assert!(journal.get_event_bytes(RUN, event.seq())?.is_some());
    }
    let before = fault.expected_after_injection()?;
    let index: Vec<_> = before
        .iter()
        .filter(|(name, _, _)| name == "index_action")
        .collect();
    let key = vb_storage::keys::index_action_key(case.ticket.action, RUN, case.ticket.step)?;
    assert_eq!(
        index,
        vec![&(String::from("index_action"), key.to_vec(), Vec::new())]
    );
    drop(live);
    drop(fault);
    drop(journal);
    let mut after = fixture::rows(case.directory.path())?;
    after.sort();
    assert_eq!(after, before);
    Ok(())
}
