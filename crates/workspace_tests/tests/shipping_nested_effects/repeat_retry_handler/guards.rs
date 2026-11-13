#![forbid(unsafe_code)]
use super::super::support::TestResult;
use super::case::{
    OTHER_RUN, OwnedCase, RETRY_ONLY_SOURCE, RUN, action_output, action_step, count, output_for,
    pending_action, retry_timer, wait_for_retry,
};
use vb_core::action::{ActionFailure, ActionFailureCode};
use vb_core::{SlotValue, Taint};
use vb_runtime::RuntimeError;
use vb_storage::JournalEvent;

#[test]
fn nested_repeat_retry_without_local_handler_reaches_public_action_ticket() -> TestResult {
    let case = OwnedCase::new_from_source(OTHER_RUN, RETRY_ONLY_SOURCE)?;
    let step = action_step(&case)?;
    let bound = usize::from(case.workflow.node_count())
        .checked_mul(64)
        .ok_or("drive bound")?;
    let (journal, mut runtime) = case.activate()?;
    let ticket = pending_action(&mut runtime, &journal, &case, step, bound)?;
    assert_eq!(ticket.run, OTHER_RUN);
    assert_eq!(ticket.step, step);
    assert_eq!(ticket.attempt, 1);
    assert_eq!(
        count(&journal.events_for_run_full(OTHER_RUN)?, |event| matches!(
            event,
            JournalEvent::ActionScheduledTicket { .. }
        )),
        1
    );
    Ok(())
}

#[test]
fn nested_retry_rejects_foreign_ticket_retargeted_to_live_run_without_mutation() -> TestResult {
    let case = OwnedCase::new(RUN)?;
    let foreign = OwnedCase::new_from_source(OTHER_RUN, RETRY_ONLY_SOURCE)?;
    let step = action_step(&case)?;
    let foreign_step = action_step(&foreign)?;
    let bound = usize::from(case.workflow.node_count())
        .checked_mul(64)
        .ok_or("drive bound")?;
    let (journal, mut runtime) = case.activate()?;
    let (foreign_journal, mut foreign_runtime) = foreign.activate()?;
    let ticket = pending_action(&mut runtime, &journal, &case, step, bound)?;
    let foreign_ticket = pending_action(
        &mut foreign_runtime,
        &foreign_journal,
        &foreign,
        foreign_step,
        bound,
    )?;
    assert_eq!(ticket.run, RUN);
    assert_eq!(foreign_ticket.run, OTHER_RUN);
    assert_ne!(ticket, foreign_ticket);
    let before_foreign_frame = runtime.inspect_run_state(RUN)?.frame.checkpoint_v1();
    let before_foreign_store = runtime.inspect_run_state(RUN)?.store.checkpoint_v1();
    let before_foreign_events = journal.events_for_run_full(RUN)?;
    let retargeted = vb_core::ActionTicket {
        run: RUN,
        ..foreign_ticket
    };
    assert_eq!(
        runtime.complete_action_with_output(retargeted, output_for(action_output(&case)?, 7)?),
        Err(RuntimeError::InvalidActionCompletion)
    );
    assert_eq!(
        runtime.inspect_run_state(RUN)?.frame.checkpoint_v1(),
        before_foreign_frame
    );
    assert_eq!(
        runtime.inspect_run_state(RUN)?.store.checkpoint_v1(),
        before_foreign_store
    );
    assert_eq!(journal.events_for_run_full(RUN)?, before_foreign_events);
    Ok(())
}

#[test]
fn nested_retry_rejects_duplicate_action_ticket_without_mutation() -> TestResult {
    let case = OwnedCase::new(RUN)?;
    let step = action_step(&case)?;
    let bound = usize::from(case.workflow.node_count())
        .checked_mul(64)
        .ok_or("drive bound")?;
    let (journal, mut runtime) = case.activate()?;
    let ticket = pending_action(&mut runtime, &journal, &case, step, bound)?;
    runtime.complete_action_with_output(ticket, output_for(action_output(&case)?, 41)?)?;
    runtime.tick_all()?;
    let second = pending_action(&mut runtime, &journal, &case, step, bound)?;
    assert_ne!(second, ticket);
    let before_duplicate_frame = runtime.inspect_run_state(RUN)?.frame.checkpoint_v1();
    let before_duplicate_store = runtime.inspect_run_state(RUN)?.store.checkpoint_v1();
    let before_duplicate_events = journal.events_for_run_full(RUN)?;
    assert_eq!(
        runtime.complete_action_with_output(ticket, output_for(action_output(&case)?, 99)?),
        Ok(())
    );
    assert_eq!(
        runtime.tick_all(),
        Err(RuntimeError::InvalidActionCompletion)
    );
    assert_eq!(
        runtime.inspect_run_state(RUN)?.frame.checkpoint_v1(),
        before_duplicate_frame
    );
    assert_eq!(
        runtime.inspect_run_state(RUN)?.store.checkpoint_v1(),
        before_duplicate_store
    );
    assert_eq!(journal.events_for_run_full(RUN)?, before_duplicate_events);
    Ok(())
}

#[test]
fn nested_retry_exhaustion_executes_local_handler_before_next_repeat_activation() -> TestResult {
    let case = OwnedCase::new(RUN)?;
    let step = action_step(&case)?;
    let bound = usize::from(case.workflow.node_count())
        .checked_mul(64)
        .ok_or("drive bound")?;
    let (journal, mut runtime) = case.activate()?;
    let first = pending_action(&mut runtime, &journal, &case, step, bound)?;
    runtime.fail_action(first, ActionFailure::from(ActionFailureCode::Timeout))?;
    runtime.tick_all()?;
    let events = journal.events_for_run_full(case.run)?;
    let timer = retry_timer(&events, first)?;
    let retry = wait_for_retry(&mut runtime, &journal, &case, step, &timer)?;
    runtime.fail_action(retry, ActionFailure::from(ActionFailureCode::Timeout))?;
    runtime.tick_all()?;
    let handler = match case.workflow.to_parts().nodes[step.as_usize()]
        .routing
        .error
    {
        vb_core::workflow::ErrorRouting::Handler { handler, .. } => handler,
        other => return Err(format!("local action handler missing: {other:?}").into()),
    };
    let third = pending_action(&mut runtime, &journal, &case, step, bound)?;
    assert_ne!(third.seq, retry.seq);
    let events = journal.events_for_run_full(case.run)?;
    assert!(events.iter().any(|event| matches!(event, JournalEvent::StepSucceeded { step: succeeded, .. } if *succeeded == handler)));
    runtime.complete_action_with_output(third, output_for(action_output(&case)?, 43)?)?;
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
            JournalEvent::RetryTimerScheduled { .. }
        )),
        1
    );
    assert_eq!(
        count(&events, |event| matches!(
            event,
            JournalEvent::ActionCompletedEnvelope { .. }
        )),
        1
    );
    assert_eq!(
        count(&events, |event| matches!(
            event,
            JournalEvent::RunFinished { .. }
        )),
        1
    );
    assert_eq!(
        super::super::support::run_finished(&events)?,
        (SlotValue::I64(42), Taint::Clean)
    );
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    Ok(())
}
