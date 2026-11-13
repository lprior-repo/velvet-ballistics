#![forbid(unsafe_code)]
//! Live action failure routing and handler-state cold recovery.

use super::support::{Case, RUN, TestResult, action_step, handler_step};
use vb_core::action::ActionFailureCode;
use vb_core::{SlotValue, StepState, Taint};
use vb_runtime::RuntimeError;
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::shard::{AskAnswer, AskTicket};
use vb_storage::JournalEvent;

#[test]
fn action_failed_before_handler_entry_reopens_the_exact_live_cut() -> TestResult {
    let case = Case::new(None, true)?;
    let action = action_step(&case.workflow)?;
    let handler = handler_step(&case.workflow)?;
    let (journal, mut runtime) = case.activate_before_handler_cut()?;
    let ticket = pending_action(&runtime, action)?;
    assert_eq!(
        (ticket.run_attempt, ticket.attempt, ticket.capacity),
        (1, 1, 1)
    );
    runtime.fail_action(ticket, ActionFailureCode::InvalidInput.into())?;
    assert_eq!(runtime.tick_all(), Err(RuntimeError::JournalPoisoned));
    let prefix = case.events(&journal)?;
    assert_failure_cut(&prefix, action, handler, false)?;
    let state = runtime.inspect_run_state(RUN)?;
    assert_eq!(state.frame.step_state(action)?, StepState::Failed);
    assert_eq!(state.frame.step_state(handler)?, StepState::Pending);
    assert_eq!(state.frame.parallel_in_flight(), 2);
    let live_frame = state.frame.checkpoint_v1();
    let live_store = state.store.checkpoint_v1();
    drop(runtime);
    drop(journal);

    let (journal, runtime) = case.open()?;
    let product = runtime.recover_product(RUN)?;
    let resumable = match product {
        RuntimeRecoveryProduct::Resumable(product) => product,
        other => return Err(format!("ActionFailed prefix was rejected: {other:?}").into()),
    };
    runtime.recover_and_resume(RUN)?;
    let recovered = runtime.inspect_run_state(RUN)?;
    assert_eq!(recovered.frame.checkpoint_v1(), live_frame);
    assert_eq!(recovered.store.checkpoint_v1(), live_store);
    assert_eq!(recovered.frame.step_state(action)?, StepState::Failed);
    assert_eq!(recovered.frame.parallel_in_flight(), 2);
    assert_eq!(resumable.run_id(), RUN);
    assert_eq!(scheduled_count(&case.events(&journal)?, action), 1);
    Ok(())
}

#[test]
fn started_handler_ask_reopens_before_group_continuation() -> TestResult {
    let case = Case::new_ask_handler()?;
    let action = action_step(&case.workflow)?;
    let handler = handler_step(&case.workflow)?;
    let (journal, mut runtime) = case.activate()?;
    runtime.fail_action(
        pending_action(&runtime, action)?,
        ActionFailureCode::InvalidInput.into(),
    )?;
    let ask = pending_ask(&mut runtime, handler)?;
    let prefix = case.events(&journal)?;
    assert_failure_cut(&prefix, action, handler, true)?;
    assert_eq!(
        runtime.inspect_run_state(RUN)?.frame.step_state(handler)?,
        StepState::Asking
    );
    assert_eq!(
        runtime.inspect_run_state(RUN)?.frame.parallel_in_flight(),
        2
    );
    let live_frame = runtime.inspect_run_state(RUN)?.frame.checkpoint_v1();
    let live_store = runtime.inspect_run_state(RUN)?.store.checkpoint_v1();
    drop(runtime);
    drop(journal);

    let (journal, mut runtime) = case.open()?;
    let product = runtime.recover_product(RUN)?;
    if !matches!(product, RuntimeRecoveryProduct::Resumable(_)) {
        return Err(format!("started-handler prefix was rejected: {product:?}").into());
    }
    runtime.recover_and_resume(RUN)?;
    let recovered = runtime.inspect_run_state(RUN)?;
    assert_eq!(recovered.frame.checkpoint_v1(), live_frame);
    assert_eq!(recovered.store.checkpoint_v1(), live_store);
    assert_eq!(recovered.frame.step_state(handler)?, StepState::Asking);
    assert_eq!(pending_ask(&mut runtime, handler)?, ask);
    assert_eq!(scheduled_count(&case.events(&journal)?, action), 1);
    runtime.answer_ask(AskAnswer::new(
        ask,
        ask.answer_slot(),
        SlotValue::I64(23),
        Taint::Clean,
    ))?;
    for _ in 0..64 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(scheduled_count(&case.events(&journal)?, action), 1);
    Ok(())
}

fn pending_action(
    runtime: &vb_runtime::runtime::Runtime,
    step: vb_core::StepIdx,
) -> TestResult<vb_core::action::ActionTicket> {
    runtime
        .lookup_pending_action_ticket(RUN, u64::from(step.get()))
        .ok_or_else(|| "pending branch action missing".into())
}

fn pending_ask(
    runtime: &mut vb_runtime::runtime::Runtime,
    step: vb_core::StepIdx,
) -> TestResult<AskTicket> {
    for _ in 0..64 {
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(RUN, step) {
            return Ok(ticket);
        }
        runtime.tick_all()?;
    }
    Err("handler Ask boundary missing".into())
}

fn assert_failure_cut(
    events: &[JournalEvent],
    action: vb_core::StepIdx,
    handler: vb_core::StepIdx,
    started: bool,
) -> TestResult {
    let failed = events
        .iter()
        .position(|event| matches!(event, JournalEvent::ActionFailedEvent { run, step, attempt: 1, .. } if *run == RUN && *step == action))
        .ok_or("ActionFailed evidence missing")?;
    let route_evidence = events.get(failed.checked_add(1).ok_or("event index overflow")?);
    assert_eq!(
        matches!(route_evidence, Some(JournalEvent::StepStarted { step, .. }) if *step == handler),
        started,
        "durable cut must match handler entry"
    );
    Ok(())
}

fn scheduled_count(events: &[JournalEvent], step: vb_core::StepIdx) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { ticket, .. } if ticket.run == RUN && ticket.step == step))
        .count()
}
