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
    let case = Case::new_ask_handler()?;
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
    drop(runtime);
    drop(journal);

    let reference = Case::new_ask_handler()?;
    let (reference_journal, mut reference_runtime) = reference.activate()?;
    let reference_action = action_step(&reference.workflow)?;
    reference_runtime.fail_action(
        pending_action(&reference_runtime, reference_action)?,
        ActionFailureCode::InvalidInput.into(),
    )?;
    let reference_ask = pending_ask(&mut reference_runtime, handler_step(&reference.workflow)?)?;
    let reference_state = reference_runtime.inspect_run_state(RUN)?;
    let reference_frame = reference_state.frame.checkpoint_v1();
    let reference_store = reference_state.store.checkpoint_v1();

    let (journal, mut runtime) = case.open()?;
    let product = runtime
        .recover_product(RUN)
        .map_err(|error| format!("recover_product failed: {error:?}"))?;
    let resumable = match product {
        RuntimeRecoveryProduct::Resumable(product) => product,
        other => return Err(format!("ActionFailed prefix was rejected: {other:?}").into()),
    };
    runtime
        .recover_and_resume(RUN)
        .map_err(|error| format!("recover_and_resume failed: {error:?}"))?;
    let recovered_ask = pending_ask(&mut runtime, handler)?;
    let recovered = runtime
        .inspect_run_state(RUN)
        .map_err(|error| format!("inspect after recovery failed: {error:?}"))?;
    assert_eq!(recovered.frame.checkpoint_v1(), reference_frame);
    assert_eq!(recovered.store.checkpoint_v1(), reference_store);
    assert_eq!(recovered.frame.step_state(action)?, StepState::Failed);
    assert_eq!(recovered.frame.parallel_in_flight(), 2);
    assert_eq!(recovered_ask, reference_ask);
    assert_eq!(resumable.run_id(), RUN);
    assert_eq!(scheduled_count(&case.events(&journal)?, action), 1);
    assert_eq!(
        scheduled_count(&reference.events(&reference_journal)?, reference_action),
        1
    );
    drop(reference_runtime);
    drop(reference_journal);
    Ok(())
}

#[test]
fn nonretryable_unhandled_branch_failure_stays_terminal_after_cold_reopen() -> TestResult {
    let case = Case::new(None, false)?;
    let action = action_step(&case.workflow)?;
    let (journal, mut runtime) = case.activate()?;
    runtime.fail_action(
        pending_action(&runtime, action)?,
        ActionFailureCode::InvalidInput.into(),
    )?;
    runtime.tick_all()?;
    let prefix = case.events(&journal)?;
    let failure_index = prefix
        .iter()
        .position(|event| matches!(event, JournalEvent::ActionFailedEvent { run, step, attempt: 1, .. } if *run == RUN && *step == action))
        .ok_or("nonretryable ActionFailed event missing")?;
    assert!(matches!(
        prefix.get(failure_index.checked_add(1).ok_or("event index overflow")?),
        Some(JournalEvent::RunFailedEvent { run, attempt: 1, .. }) if *run == RUN
    ));
    assert_eq!(scheduled_count(&prefix, action), 1);
    drop(runtime);
    drop(journal);

    let (journal, runtime) = case.open()?;
    let product = runtime.recover_product(RUN)?;
    assert_eq!(product.cannot_resume_reason(), Some("terminal_state"));
    assert_eq!(
        runtime.recover_and_resume(RUN),
        Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("terminal_state")
        })
    );
    assert_eq!(case.events(&journal)?, prefix);
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
    let product = runtime
        .recover_product(RUN)
        .map_err(|error| format!("recover_product failed: {error:?}"))?;
    if !matches!(product, RuntimeRecoveryProduct::Resumable(_)) {
        return Err(format!("started-handler prefix was rejected: {product:?}").into());
    }
    runtime
        .recover_and_resume(RUN)
        .map_err(|error| format!("recover_and_resume failed: {error:?}"))?;
    runtime.tick_all()?;
    let recovered = runtime
        .inspect_run_state(RUN)
        .map_err(|error| format!("inspect after recovery failed: {error:?}"))?;
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
    let completed = case.events(&journal)?;
    let join = together_join_step(&case.workflow)?;
    assert!(completed.iter().any(
        |event| matches!(event, JournalEvent::StepSucceeded { run, step, .. } if *run == RUN && *step == join)
    ));
    assert!(
        completed
            .iter()
            .any(|event| matches!(event, JournalEvent::RunFinished { run, .. } if *run == RUN))
    );
    assert_eq!(scheduled_count(&completed, action), 1);
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

fn together_join_step(workflow: &vb_core::CompiledWorkflow) -> TestResult<vb_core::StepIdx> {
    workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| {
            matches!(node.kind, vb_core::CompiledNodeKind::TogetherJoin { .. }).then_some(node.id)
        })
        .ok_or_else(|| "TogetherJoin node missing".into())
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
