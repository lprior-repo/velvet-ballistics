use super::{RUN, TestResult, batch_shape, cut, fixture};
use vb_runtime::{
    RuntimeError,
    journal::{RuntimeJournal, RuntimeJournalEvent},
};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

type Rows = Vec<(String, Vec<u8>, Vec<u8>)>;

pub(super) fn wire_charges(
    journal: &cut::CutJournal,
    seq: EventSeq,
    events: &[RuntimeJournalEvent],
) -> TestResult<Vec<u64>> {
    events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let sequence = EventSeq::new(
                seq.get()
                    .checked_add(u64::try_from(index)?)
                    .ok_or("sequence overflow")?,
            );
            let stored = journal.convert_to_storage_event(event.clone(), sequence)?;
            Ok(u64::try_from(postcard::to_allocvec(&stored)?.len())?)
        })
        .collect()
}

pub(super) fn tight_case() -> TestResult<(fixture::Case, Vec<RuntimeJournalEvent>)> {
    let control = fixture::Case::new(false, false)?;
    let (_, fault, mut live) = control.open(cut::Mode::Observe)?;
    let state = live.inspect_run_state(RUN)?;
    let before = state.journal_bytes_written;
    let budget = state
        .admission
        .as_ref()
        .and_then(|a| a.budget())
        .ok_or("real budget absent")?;
    let units = budget
        .max_journal_bytes_total
        .checked_div(u64::from(
            state.workflow.resource_contract().max_journal_batch_bytes,
        ))
        .ok_or("zero batch units")?;
    live.fail_action(control.ticket, fixture::failure())?;
    live.tick_all()?;
    let (seq, events) = fault.observed_facts()?;
    batch_shape(&events, control.ticket)?;
    let stored = fault.journal().events_for_run_full(RUN)?;
    for (index, event) in events.iter().enumerate() {
        let at = EventSeq::new(
            seq.get()
                .checked_add(u64::try_from(index)?)
                .ok_or("sequence overflow")?,
        );
        let converted = fault.convert_to_storage_event(event.clone(), at)?;
        assert_eq!(
            stored.iter().find(|record| record.seq() == at),
            Some(&converted)
        );
    }
    let charges = wire_charges(&fault, seq, &events)?;
    let failure = *charges.first().ok_or("failure charge absent")?;
    let total = charges.iter().try_fold(0_u64, |sum, next| {
        sum.checked_add(*next).ok_or("wire charge overflow")
    })?;
    let lower = before.checked_add(failure).ok_or("lower overflow")?;
    let upper = before
        .checked_add(total)
        .and_then(|v| v.checked_sub(1))
        .ok_or("upper overflow")?;
    let middle = lower
        .checked_add(
            upper
                .checked_sub(lower)
                .ok_or("empty budget interval")?
                .checked_div(2)
                .ok_or("division failed")?,
        )
        .ok_or("midpoint overflow")?;
    let limit = u32::try_from(middle.checked_div(units).ok_or("zero budget units")?)?;
    let case = fixture::Case::with_journal_batch_limit(limit)?;
    Ok((case, events))
}

fn actual_tight_boundary(
    case: &fixture::Case,
    fault: &cut::CutJournal,
    live: &vb_runtime::runtime::Runtime,
    facts: &[RuntimeJournalEvent],
) -> TestResult {
    batch_shape(facts, case.ticket)?;
    let state = live.inspect_run_state(RUN)?;
    let limit = state
        .admission
        .as_ref()
        .and_then(|a| a.budget())
        .ok_or("real budget absent")?
        .max_journal_bytes_total;
    let next = fault
        .journal()
        .events_for_run_full(RUN)?
        .last()
        .ok_or("prefix absent")?
        .seq()
        .get()
        .checked_add(1)
        .ok_or("sequence overflow")?;
    let charges = wire_charges(fault, EventSeq::new(next), facts)?;
    let failed = *charges.first().ok_or("failure charge absent")?;
    let all = charges.iter().try_fold(0_u64, |sum, next| {
        sum.checked_add(*next).ok_or("charge overflow")
    })?;
    assert!(
        state
            .journal_bytes_written
            .checked_add(failed)
            .ok_or("charge overflow")?
            <= limit
    );
    assert!(
        state
            .journal_bytes_written
            .checked_add(all)
            .ok_or("charge overflow")?
            > limit
    );
    assert_eq!(
        state.workflow.resource_contract().max_journal_batch_bytes,
        case.batch_limit()
    );
    Ok(())
}

#[test]
fn tight_checked_contract_commits_failure_and_terminal_together() -> TestResult {
    let (case, facts) = tight_case()?;
    let (journal, fault, mut live) = case.open(cut::Mode::Observe)?;
    actual_tight_boundary(&case, &fault, &live, &facts)?;
    let prefix = journal.events_for_run_full(RUN)?;
    let receipt = case.receipt(&journal)?;
    live.fail_action(case.ticket, fixture::failure())?;
    live.tick_all()?;
    let (_, batch) = fault.captured()?;
    assert!(
        matches!(batch.as_slice(), [RuntimeJournalEvent::ActionFailed { run, step, action, attempt }, RuntimeJournalEvent::RunFailed { run: terminal, attempt: run_attempt }] if *run==RUN && *terminal==RUN && *step==case.ticket.step && *action==case.ticket.action && *attempt==case.ticket.attempt && *run_attempt==case.ticket.run_attempt)
    );
    assert_eq!(live.counters_snapshot().runs_failed, 1);
    assert_eq!(
        live.inspect_run_state(RUN).cloned(),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(live.lookup_pending_action_ticket(RUN, 0), None);
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(events.get(..prefix.len()), Some(prefix.as_slice()));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, JournalEvent::RunFailedEvent { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        1
    );
    assert_eq!(case.receipt(&journal)?, receipt);
    drop(live);
    drop(fault);
    drop(journal);
    let cold = FjallJournal::open(case.directory.path(), None)?;
    assert_eq!(cold.events_for_run_full(RUN)?, events);
    assert_eq!(case.receipt(&cold)?, receipt);
    let key = vb_storage::keys::index_action_key(case.ticket.action, RUN, case.ticket.step)?;
    assert!(!cold.has_action_index_entry(&key)?);
    Ok(())
}

#[test]
fn tight_checked_contract_terminal_append_refusal_preserves_original_state() -> TestResult {
    let (case, facts) = tight_case()?;
    let before: Rows = fixture::rows(case.directory.path())?;
    let (journal, fault, mut live) = case.open(cut::Mode::Reject)?;
    actual_tight_boundary(&case, &fault, &live, &facts)?;
    let state = live.inspect_run_state(RUN)?.clone();
    let pending = live.pending_boundary_snapshot(8);
    let counters = live.counters_snapshot();
    let quota = live.usage_snapshot();
    let trace = live.list_events(RUN)?;
    let events = journal.events_for_run_full(RUN)?;
    let receipt = case.receipt(&journal)?;
    live.fail_action(case.ticket, fixture::failure())?;
    assert_eq!(live.tick_all(), Err(RuntimeError::JournalPoisoned));
    assert!(matches!(
        fault.captured()?.1.as_slice(),
        [
            RuntimeJournalEvent::ActionFailed { .. },
            RuntimeJournalEvent::RunFailed { .. }
        ]
    ));
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
fn zero_tick_budget_rejects_before_commit_and_keeps_original_ticket_usable() -> TestResult {
    let case = fixture::Case::new(false, false)?;
    let before = fixture::rows(case.directory.path())?;
    let (journal, fault, mut live) = case.open_with_tick_budget(cut::Mode::Observe, 0)?;
    let state = live.inspect_run_state(RUN)?.clone();
    let pending = live.pending_boundary_snapshot(8);
    let counters = live.counters_snapshot();
    let quota = live.usage_snapshot();
    let trace = live.list_events(RUN)?;
    let events = journal.events_for_run_full(RUN)?;
    let receipt = case.receipt(&journal)?;
    live.fail_action(case.ticket, fixture::failure())?;
    assert_eq!(
        live.tick_all(),
        Err(vb_core::errors::CoreError::StepBudgetExhausted.into())
    );
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
    let (journal, _, mut recovered) = case.open(cut::Mode::Observe)?;
    assert_eq!(
        recovered.complete_action_with_output(case.ticket, case.output()?),
        Ok(())
    );
    recovered.tick_all()?;
    assert_eq!(journal.events_for_run_full(RUN)?.iter().filter(|e| matches!(e, JournalEvent::ActionCompletedEnvelope { ticket, .. } if *ticket==case.ticket)).count(), 1);
    Ok(())
}
