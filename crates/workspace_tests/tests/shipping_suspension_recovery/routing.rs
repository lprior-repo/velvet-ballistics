//! Error-routing replay and recovery scenarios: the unrouted `ActionFailed`
//! gap and the already-started handler window.

use vb_core::action::ActionFailureCode;
use vb_core::{SlotValue, StepState, Taint};
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::shard::AskAnswer;
use vb_storage::JournalEvent;

use super::support::{
    RUN, TestResult, action_step, ask_step, contract, drain_until_completed, finish_result_slot,
    live_routed_failure_prefix, open, pending_ask, recorded_slot_writes, routed_failure_source,
    scratch_store, submit,
};

/// Given a live run whose action failure routed to its `on_error` ask handler,
/// When the durable prefix ends at the `ActionFailed` append and the store is
/// reopened,
/// Then recovery restores the handler jump, does not reissue the action, and
/// the suspension completes with the authored result.
#[test]
fn recovered_unrouted_action_failure_restores_the_handler_without_reissuing_the_action()
-> TestResult {
    let workflow = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&workflow)?;
    let ask = ask_step(&workflow)?;
    let finish = finish_result_slot(&workflow)?;
    let (prefix, live_ticket, inputs) = live_routed_failure_prefix(&workflow, ask)?;
    let (_directory, journal, mut runtime) =
        scratch_store(&workflow, &prefix, &[contract()?], &inputs)?;

    assert!(matches!(
        runtime.recover_product(RUN)?,
        RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    let ticket = pending_ask(&mut runtime, ask)?;
    assert_eq!(
        ticket, live_ticket,
        "recovery must preserve the recorded ask boundary identity"
    );
    let recovered = runtime.inspect_run_state(RUN)?;
    assert_eq!(
        recovered.frame.step_state(work)?,
        StepState::Failed,
        "recovered failure must keep the failed step state"
    );
    assert_eq!(
        recovered.frame.step_state(ask)?,
        StepState::Asking,
        "recovered run must resume inside the authored handler"
    );
    let tail = journal
        .events_for_run_full(RUN)?
        .get(prefix.len()..)
        .ok_or("tail range")?
        .to_vec();
    assert_eq!(
        tail.iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        0,
        "recovery must not reissue the resolved action"
    );
    assert!(
        !tail
            .iter()
            .any(|event| matches!(event, JournalEvent::StepStarted { step, .. } if *step == work)),
        "recovery must not re-drive the failed action step"
    );

    runtime.answer_ask(AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(99),
        Taint::Clean,
    ))?;
    drain_until_completed(&mut runtime)?;
    let finished = journal
        .events_for_run_full(RUN)?
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or("handler must finish the recovered run")?;
    assert_eq!(finished, finish);
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(
        tail.iter()
            .filter(|event| matches!(
                event,
                JournalEvent::RunAdmission { .. } | JournalEvent::RunAdmissionBudget { .. }
            ))
            .count(),
        0,
        "recovery must not re-evaluate the recorded admission policy"
    );
    assert_eq!(
        journal.events_for_run_full(RUN)?.get(..prefix.len()),
        Some(prefix.as_slice()),
        "recovery must not rewrite durable evidence"
    );
    Ok(())
}

/// Given a live run that already journaled the handler's `StepStarted` after
/// the action failure, When the store is reopened from that prefix, Then
/// recovery resumes inside the already-started handler, replays the failed step
/// as `Failed`, journals no duplicate handler entry, does not reissue the
/// action, and the handler's ask completes the run.
#[test]
fn recovered_started_handler_resumes_without_reissuing_the_action() -> TestResult {
    let directory = tempfile::tempdir()?;
    let workflow = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&workflow)?;
    let ask = ask_step(&workflow)?;
    let answer_slot = workflow
        .node(ask)
        .ok_or("handler Ask node")?
        .output
        .slot()
        .ok_or("handler Ask answer slot")?;
    let handler = workflow
        .node(work)
        .ok_or("work step node")?
        .routing
        .error
        .handler()
        .ok_or("workflow must route the failure")?;
    let (prefix, inputs) = {
        let (journal, mut runtime) = open(directory.path())?;
        submit(&journal, &mut runtime, &workflow, &[contract()?])?;
        let ticket = runtime
            .lookup_pending_action_ticket(RUN, u64::from(work.get()))
            .ok_or("scheduled action ticket")?;
        runtime.fail_action(ticket, ActionFailureCode::InvalidInput.into())?;
        pending_ask(&mut runtime, ask)?;
        let inputs = journal.run_inputs(RUN)?.ok_or("retained inputs absent")?;
        let events = journal.events_for_run_full(RUN)?;
        let failed = events
            .iter()
            .position(|event| matches!(event, JournalEvent::ActionFailedEvent { .. }))
            .ok_or("ActionFailed evidence")?;
        let started = events
            .iter()
            .enumerate()
            .skip(failed)
            .find_map(|(index, event)| match event {
                JournalEvent::StepStarted { step, .. } if *step == handler => Some(index),
                _ => None,
            })
            .ok_or("handler StepStarted evidence")?;
        (
            events.get(..=started).ok_or("prefix range")?.to_vec(),
            inputs,
        )
    };

    let (_scratch, journal, mut runtime) =
        scratch_store(&workflow, &prefix, &[contract()?], &inputs)?;
    assert!(matches!(
        runtime.recover_product(RUN)?,
        RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    runtime.tick_all()?;
    let ticket = pending_ask(&mut runtime, ask)?;
    let recovered = runtime.inspect_run_state(RUN)?;
    assert_eq!(
        recovered.frame.step_state(work)?,
        StepState::Failed,
        "the journal-authorized failure must replay as Failed"
    );
    assert_eq!(
        recovered.frame.step_state(ask)?,
        StepState::Asking,
        "recovered run must resume inside the already-started handler"
    );
    assert_eq!(
        recovered.frame.executed(),
        1,
        "recovery executes the handler body once; its StepStarted was already durable"
    );
    let events = journal.events_for_run_full(RUN)?;
    let tail = events.get(prefix.len()..).ok_or("tail range")?;
    assert!(
        !tail.iter().any(|event| matches!(
            event,
            JournalEvent::RunFailedEvent { .. } | JournalEvent::RetryCheckpoint { .. }
        )),
        "an already-started handler must resume, not terminalize the run"
    );
    assert_eq!(
        tail.iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        0,
        "an already-started handler must not reissue the action"
    );
    assert_eq!(
        tail.iter()
            .filter(|event| matches!(event, JournalEvent::AskScheduledEvent { .. }))
            .count(),
        1,
        "the handler ask must suspend exactly once"
    );
    assert_eq!(
        events
            .iter()
            .filter(
                |event| matches!(event, JournalEvent::StepStarted { step, .. } if *step == handler)
            )
            .count(),
        1,
        "the journal-authorized handler entry must not be re-journaled"
    );
    runtime.answer_ask(AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(99),
        Taint::Clean,
    ))?;
    drain_until_completed(&mut runtime)?;
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = journal.events_for_run_full(RUN)?;
    let answered_seq = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::AskAnsweredEvent {
                run,
                seq,
                step,
                attempt,
            } if *run == RUN && *step == ask && *attempt == 1 => Some(*seq),
            _ => None,
        })
        .ok_or("durable handler AskAnswered event missing")?;
    let answer_writes = recorded_slot_writes(&events, answer_slot)?;
    assert_eq!(
        answer_writes.len(),
        2,
        "handler answer slot must preserve both durable writes"
    );
    assert!(answer_writes.iter().all(|(_, value, taint, attempt)| {
        *value == SlotValue::I64(99) && *taint == Taint::Clean && *attempt == 1
    }));
    assert_eq!(
        answer_writes.first().map(|(seq, _, _, _)| seq.get()),
        Some(
            answered_seq
                .get()
                .checked_sub(1)
                .ok_or("answer seq underflow")?
        ),
        "the answer value write immediately precedes AskAnswered"
    );
    let finished = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or("resumed handler must finish the run")?;
    assert_eq!(finished, finish_result_slot(&workflow)?);
    assert!(events.iter().any(|event| matches!(
        event,
        JournalEvent::RunFinished { run, result, attempt, .. }
            if *run == RUN && *result == finished && *attempt == 1
    )));
    let result_writes = recorded_slot_writes(&events, finished)?;
    assert_eq!(result_writes.len(), 1);
    assert!(result_writes.iter().all(|(_, value, taint, attempt)| {
        *value == SlotValue::I64(42) && *taint == Taint::Clean && *attempt == 1
    }));
    Ok(())
}
