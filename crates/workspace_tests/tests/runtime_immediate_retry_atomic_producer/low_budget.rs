use super::{RUN, TestResult, budgets, cut, fixture};
use vb_runtime::{RuntimeError, journal::RuntimeJournalEvent};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

fn below_failure_case() -> TestResult<(fixture::Case, Vec<RuntimeJournalEvent>)> {
    let (control, facts) = budgets::tight_case()?;
    let (_, fault, live) = control.open(cut::Mode::Observe)?;
    let state = live.inspect_run_state(RUN)?;
    let budget = state
        .admission
        .as_ref()
        .and_then(|a| a.budget())
        .ok_or("budget absent")?;
    let units = budget
        .max_journal_bytes_total
        .checked_div(u64::from(control.batch_limit()))
        .ok_or("zero units")?;
    let next = fault
        .journal()
        .events_for_run_full(RUN)?
        .last()
        .ok_or("prefix absent")?
        .seq()
        .get()
        .checked_add(1)
        .ok_or("sequence overflow")?;
    let charges = budgets::wire_charges(&fault, EventSeq::new(next), &facts)?;
    let failure = *charges.first().ok_or("failure charge absent")?;
    let bound = state
        .journal_bytes_written
        .checked_add(failure)
        .and_then(|n| n.checked_sub(1))
        .ok_or("bound overflow")?;
    let limit = u32::try_from(bound.checked_div(units).ok_or("zero units")?)?;
    Ok((fixture::Case::with_journal_batch_limit(limit)?, facts))
}

#[test]
fn below_action_failure_budget_preserves_terminal_only_policy_cold() -> TestResult {
    let (case, facts) = below_failure_case()?;
    let (journal, fault, mut live) = case.open(cut::Mode::Observe)?;
    let prefix = journal.events_for_run_full(RUN)?;
    let receipt = case.receipt(&journal)?;
    let state = live.inspect_run_state(RUN)?;
    let limit = state
        .admission
        .as_ref()
        .and_then(|a| a.budget())
        .ok_or("budget absent")?
        .max_journal_bytes_total;
    let seq = EventSeq::new(
        prefix
            .last()
            .ok_or("prefix absent")?
            .seq()
            .get()
            .checked_add(1)
            .ok_or("sequence overflow")?,
    );
    let charges = budgets::wire_charges(&fault, seq, &facts)?;
    assert!(state.journal_bytes_written <= limit);
    assert!(
        state
            .journal_bytes_written
            .checked_add(*charges.first().ok_or("failure charge absent")?)
            .ok_or("charge overflow")?
            > limit
    );
    live.fail_action(case.ticket, fixture::failure())?;
    live.tick_all()?;
    assert!(
        matches!(fault.captured()?.1.as_slice(), [RuntimeJournalEvent::RunFailed { run, attempt }] if *run==RUN && *attempt==case.ticket.run_attempt)
    );
    assert_eq!(
        live.inspect_run_state(RUN).cloned(),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(live.lookup_pending_action_ticket(RUN, 0), None);
    assert_eq!(live.counters_snapshot().runs_failed, 1);
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(events.get(..prefix.len()), Some(prefix.as_slice()));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, JournalEvent::ActionFailedEvent { .. }))
            .count(),
        0
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, JournalEvent::RunFailedEvent { .. }))
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
    Ok(())
}
