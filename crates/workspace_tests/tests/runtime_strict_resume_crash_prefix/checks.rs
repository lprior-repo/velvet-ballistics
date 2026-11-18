use super::{RUN, TestResult, fixture, rows, runtime};
use std::{path::Path, sync::Arc};
use vb_core::{SlotValue, StepState};
use vb_runtime::{RuntimeError, journal::StorageRuntimeJournal, runtime::Runtime};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, RunInputsRecord};

type Rows = [(String, Vec<u8>, Vec<u8>)];

pub(super) fn empty_action_index(rows: &Rows) {
    let entries: Vec<_> = rows
        .iter()
        .filter(|(name, _, _)| name == "index_action")
        .collect();
    assert_eq!(entries, Vec::<&(String, Vec<u8>, Vec<u8>)>::new());
}

pub(super) fn sequence_and_effects(events: &[JournalEvent], resumes: usize) -> TestResult {
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event.seq(), EventSeq::new(u64::try_from(index)?));
    }
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, JournalEvent::RunResumed { .. }))
            .count(),
        resumes
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, JournalEvent::ActionCompletedEnvelope { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, JournalEvent::RunFinished { .. }))
            .count(),
        0
    );
    Ok(())
}

pub(super) fn live_boundary(
    recovered: &Runtime,
    reference: &Runtime,
    issued: fixture::Issued,
) -> TestResult {
    let actual = recovered.inspect_run_state(RUN)?;
    let expected = reference.inspect_run_state(RUN)?;
    assert_eq!(actual.frame, expected.frame);
    assert_eq!(actual.store, expected.store);
    assert_eq!(actual.action_attempts, expected.action_attempts);
    assert_eq!(actual.frame.read_slot(issued.output)?, &SlotValue::I64(73));
    assert_eq!(actual.frame.read_taint(issued.output)?, issued.ready.taint);
    assert_eq!(
        actual.frame.step_state(issued.ticket.step)?,
        StepState::Succeeded
    );
    assert_eq!(actual.frame.step_state(issued.ask)?, StepState::Asking);
    assert_eq!(
        recovered.lookup_pending_action_ticket(RUN, u64::from(issued.ticket.step.get())),
        None
    );
    assert_eq!(
        recovered.pending_boundary_snapshot(8),
        reference.pending_boundary_snapshot(8)
    );
    let actual_ask = recovered
        .lookup_pending_ask_ticket(RUN, issued.ask)
        .ok_or("cold Ask authority absent")?;
    let expected_ask = reference
        .lookup_pending_ask_ticket(RUN, issued.ask)
        .ok_or("reference Ask authority absent")?;
    assert_eq!(actual_ask.run(), expected_ask.run());
    assert_eq!(actual_ask.ask_step(), expected_ask.ask_step());
    assert_eq!(actual_ask.resume_step(), expected_ask.resume_step());
    assert_eq!(actual_ask.answer_slot(), expected_ask.answer_slot());
    assert_eq!(recovered.counters_snapshot().runs_completed, 0);
    assert_eq!(
        recovered
            .pending_boundary_snapshot(8)
            .pending_action_count(),
        0
    );
    assert_eq!(
        recovered.pending_boundary_snapshot(8).pending_ask_count(),
        1
    );
    Ok(())
}

pub(super) fn old_ticket_refused_cold(
    path: &Path,
    issued: fixture::Issued,
    before_rows: &Rows,
    events: &[JournalEvent],
    receipt: &Option<RunInputsRecord>,
) -> TestResult {
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let mut recovered = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    recovered.recover_and_resume(RUN)?;
    fixture::pending_ask(&mut recovered, issued.ask)?;
    let state = recovered.inspect_run_state(RUN)?.clone();
    let ask = recovered
        .lookup_pending_ask_ticket(RUN, issued.ask)
        .ok_or("cold live Ask absent")?;
    let counters = recovered.counters_snapshot();
    let pending = recovered.pending_boundary_snapshot(8);
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    assert_eq!(journal.run_inputs(RUN)?, *receipt);
    assert_eq!(
        recovered.complete_action_with_output(issued.ticket, issued.ready),
        Err(RuntimeError::InvalidActionCompletion)
    );
    assert_eq!(recovered.inspect_run_state(RUN)?, &state);
    assert_eq!(
        recovered.lookup_pending_ask_ticket(RUN, issued.ask),
        Some(ask)
    );
    assert_eq!(
        recovered.lookup_pending_action_ticket(RUN, u64::from(issued.ticket.step.get())),
        None
    );
    assert_eq!(recovered.pending_boundary_snapshot(8), pending);
    assert_eq!(recovered.counters_snapshot(), counters);
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    assert_eq!(journal.run_inputs(RUN)?, *receipt);
    drop(recovered);
    drop(journal);
    let after = rows(path)?;
    empty_action_index(&after);
    assert_eq!(after, before_rows);
    Ok(())
}
