#![forbid(unsafe_code)]
use super::super::support::TestResult;
use super::case::{
    OwnedCase, RUN, action_output, action_step, count, output_for, pending_action, retry_timer,
    wait_for_retry,
};
use vb_core::action::{ActionFailure, ActionFailureCode};
use vb_core::{SlotValue, StepIdx, Taint};
use vb_runtime::runtime::Runtime;
use vb_storage::{FjallJournal, JournalEvent};

#[test]
fn nested_repeat_retry_reopens_and_matches_uninterrupted_reference() -> TestResult {
    let cold = OwnedCase::new(RUN)?;
    let reference = OwnedCase::new(RUN)?;
    let step = action_step(&cold)?;
    let output_slot = action_output(&cold)?;
    let bound = usize::from(cold.workflow.node_count())
        .checked_mul(64)
        .ok_or("drive bound")?;
    let (cold_journal, mut cold_runtime) = cold.activate()?;
    let (reference_journal, mut reference_runtime) = reference.activate()?;
    let first_cold = pending_action(&mut cold_runtime, &cold_journal, &cold, step, bound)?;
    let first_reference = pending_action(
        &mut reference_runtime,
        &reference_journal,
        &reference,
        step,
        bound,
    )?;
    cold_runtime.fail_action(first_cold, ActionFailure::from(ActionFailureCode::Timeout))?;
    reference_runtime.fail_action(
        first_reference,
        ActionFailure::from(ActionFailureCode::Timeout),
    )?;
    cold_runtime.tick_all()?;
    reference_runtime.tick_all()?;
    let cold_events = cold_journal.events_for_run_full(RUN)?;
    let reference_events = reference_journal.events_for_run_full(RUN)?;
    let timer = retry_timer(&cold_events, first_cold)?;
    let reference_timer = retry_timer(&reference_events, first_reference)?;
    let cold_frame = cold_runtime.inspect_run_state(RUN)?.frame.checkpoint_v1();
    let cold_store = cold_runtime.inspect_run_state(RUN)?.store.checkpoint_v1();
    assert_eq!(
        cold_frame,
        reference_runtime
            .inspect_run_state(RUN)?
            .frame
            .checkpoint_v1()
    );
    assert_eq!(
        cold_store,
        reference_runtime
            .inspect_run_state(RUN)?
            .store
            .checkpoint_v1()
    );
    assert_eq!(first_cold, first_reference);
    drop(cold_runtime);
    drop(cold_journal);

    let (cold_journal, mut cold_runtime) = cold.recover()?;
    assert_eq!(
        cold_runtime.inspect_run_state(RUN)?.frame.checkpoint_v1(),
        cold_frame
    );
    assert_eq!(
        cold_runtime.inspect_run_state(RUN)?.store.checkpoint_v1(),
        cold_store
    );
    let recovered_ticket = wait_for_retry(&mut cold_runtime, &cold_journal, &cold, step, &timer)?;
    let reference_ticket = wait_for_retry(
        &mut reference_runtime,
        &reference_journal,
        &reference,
        step,
        &reference_timer,
    )?;
    assert!(recovered_ticket.same_invocation(&reference_ticket));
    assert_eq!(
        recovered_ticket.attempt,
        first_cold
            .attempt
            .checked_add(1)
            .ok_or("attempt overflow")?
    );
    assert_eq!(recovered_ticket.idempotency_key, first_cold.idempotency_key);
    cold_runtime.complete_action_with_output(recovered_ticket, output_for(output_slot, 41)?)?;
    reference_runtime
        .complete_action_with_output(reference_ticket, output_for(output_slot, 41)?)?;
    finish_two_activations(&cold, &cold_journal, &mut cold_runtime, step, bound, 41, 43)?;
    finish_two_activations(
        &reference,
        &reference_journal,
        &mut reference_runtime,
        step,
        bound,
        41,
        43,
    )?;
    let cold_events = cold_journal.events_for_run_full(RUN)?;
    let reference_events = reference_journal.events_for_run_full(RUN)?;
    assert_eq!(event_order(&cold_events), event_order(&reference_events));
    let cold_tickets = action_tickets(&cold_events);
    let reference_tickets = action_tickets(&reference_events);
    assert_eq!(cold_tickets.len(), reference_tickets.len());
    assert!(
        cold_tickets
            .iter()
            .zip(&reference_tickets)
            .all(|(cold, reference)| cold.same_invocation(reference))
    );
    assert_eq!(
        completion_values(&cold_events)?,
        completion_values(&reference_events)?
    );
    assert_eq!(
        count(&cold_events, |event| matches!(
            event,
            JournalEvent::RunFinished { .. }
        )),
        1
    );
    assert_eq!(
        count(&cold_events, |event| matches!(
            event,
            JournalEvent::RunFailedEvent { .. }
        )),
        0
    );
    assert_eq!(
        count(&cold_events, |event| matches!(
            event,
            JournalEvent::RetryTimerScheduled { .. }
        )),
        1
    );
    assert_eq!(
        super::super::support::run_finished(&cold_events)?,
        (SlotValue::I64(42), Taint::Clean)
    );
    assert_eq!(
        super::super::support::run_finished(&reference_events)?,
        (SlotValue::I64(42), Taint::Clean)
    );
    assert_eq!(cold_runtime.counters_snapshot().runs_completed, 1);
    Ok(())
}

fn finish_two_activations(
    case: &OwnedCase,
    journal: &FjallJournal,
    runtime: &mut Runtime,
    step: StepIdx,
    bound: usize,
    first_value: i64,
    second_value: i64,
) -> TestResult {
    let second = pending_action(runtime, journal, case, step, bound)?;
    assert_ne!(second.seq, vb_core::SeqNo::ZERO);
    assert_eq!(second.capacity, 2);
    runtime.complete_action_with_output(second, output_for(action_output(case)?, second_value)?)?;
    for _ in 0..bound {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    let events = journal.events_for_run_full(case.run)?;
    assert_eq!(
        count(&events, |event| matches!(
            event,
            JournalEvent::ActionScheduledTicket { .. }
        )),
        3
    );
    assert_eq!(
        count(&events, |event| matches!(
            event,
            JournalEvent::ActionCompletedEnvelope { .. }
        )),
        2
    );
    let values = completion_values(&events)?;
    assert_eq!(values, vec![first_value, second_value]);
    Ok(())
}

fn action_tickets(events: &[JournalEvent]) -> Vec<vb_core::ActionTicket> {
    events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::ActionScheduledTicket { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .collect()
}

fn completion_values(events: &[JournalEvent]) -> TestResult<Vec<i64>> {
    events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::ActionCompletedEnvelope { value, .. } => {
                Some(postcard::from_bytes::<SlotValue>(value))
            }
            _ => None,
        })
        .map(|value| match value? {
            SlotValue::I64(value) => Ok(value),
            other => Err(format!("unexpected action completion value: {other:?}").into()),
        })
        .collect()
}

fn event_order(events: &[JournalEvent]) -> Vec<(u64, String)> {
    events
        .iter()
        .map(|event| {
            let debug = format!("{event:?}");
            let name = debug.split(['{', '(']).next().unwrap_or(&debug).to_owned();
            (event.seq().get(), name)
        })
        .collect()
}
