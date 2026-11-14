#![forbid(unsafe_code)]
//! Strict cold recovery across a durable action-retry timer.

use super::support::{Case, RUN, TestResult, action_step};
use vb_core::action::{ActionFailureCode, ActionOutputReady, ActionTicket};
use vb_core::{SlotValue, Taint};
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};
use vb_storage::JournalEvent;

#[test]
fn together_retry_timer_reopens_with_same_ticket_authority_and_completes_once() -> TestResult {
    let case = Case::new(Some((2, "10ms")), true)?;
    let step = action_step(&case.workflow)?;
    let (journal, mut runtime) = case.activate()?;
    let first_ticket = pending_action(&runtime, step)?;
    assert_eq!(
        (
            first_ticket.run_attempt,
            first_ticket.attempt,
            first_ticket.capacity
        ),
        (1, 1, 2)
    );
    runtime.fail_action(first_ticket, ActionFailureCode::Timeout.into())?;
    runtime.tick_all()?;
    let timer = runtime.capture_timer_entry(RUN)?;
    let live_state = runtime.inspect_run_state(RUN)?;
    let live_frame = live_state.frame.checkpoint_v1();
    let live_store = live_state.store.checkpoint_v1();
    assert_eq!(
        live_state.frame.step_state(step)?,
        vb_core::StepState::Waiting
    );
    let first_events = case.events(&journal)?;
    drop(runtime);
    drop(journal);

    let (journal, mut runtime) = case.open()?;
    let product = runtime.recover_product(RUN)?;
    match product {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(
                product.boundary_kind(),
                RecoveredRunBoundaryKind::RetryTimer
            );
        }
        other => return Err(format!("retry timer prefix was rejected: {other:?}").into()),
    }
    runtime.recover_and_resume(RUN)?;
    runtime.tick_all()?;
    let reopened_timer = runtime.capture_timer_entry(RUN)?;
    assert_eq!(reopened_timer.run, timer.run);
    assert_eq!(reopened_timer.generation, timer.generation);
    assert_eq!(reopened_timer.kind, timer.kind);
    let reopened_state = runtime.inspect_run_state(RUN)?;
    assert_eq!(reopened_state.frame.checkpoint_v1(), live_frame);
    assert_eq!(reopened_state.store.checkpoint_v1(), live_store);

    wait_for_timer(reopened_timer.deadline);
    runtime.timer_entry_fired(reopened_timer)?;
    runtime.tick_all()?;
    let second_ticket = pending_action(&runtime, step)?;
    assert_retry_ticket(first_ticket, second_ticket)?;
    runtime.complete_action_with_output(
        second_ticket,
        action_output(&case.workflow, second_ticket)?,
    )?;
    for _ in 0..64 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let completed = case.events(&journal)?;
    assert_eq!(
        completed
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { ticket, .. } if ticket.step == step))
            .count(),
        2
    );
    assert_eq!(
        completed
            .iter()
            .filter(|event| matches!(event, JournalEvent::RetryTimerScheduled { step: timer_step, .. } if *timer_step == step))
            .count(),
        1
    );
    assert_eq!(
        completed
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { ticket, .. } if ticket == &second_ticket))
            .count(),
        1
    );
    assert_eq!(
        first_events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RetryTimerScheduled { step: timer_step, .. } if *timer_step == step))
            .count(),
        1
    );
    Ok(())
}

fn pending_action(
    runtime: &vb_runtime::runtime::Runtime,
    step: vb_core::StepIdx,
) -> TestResult<ActionTicket> {
    runtime
        .lookup_pending_action_ticket(RUN, u64::from(step.get()))
        .ok_or_else(|| "pending retry action missing".into())
}

fn assert_retry_ticket(first: ActionTicket, second: ActionTicket) -> TestResult {
    if first.run != second.run
        || first.step != second.step
        || first.action != second.action
        || first.run_attempt != second.run_attempt
        || second.attempt != first.attempt.checked_add(1).ok_or("attempt overflow")?
        || first.capacity != second.capacity
        || first.idempotency_key != second.idempotency_key
        || first.seq != second.seq
    {
        return Err("retry ticket identity changed across the timer".into());
    }
    Ok(())
}

fn action_output(
    workflow: &vb_core::CompiledWorkflow,
    ticket: ActionTicket,
) -> TestResult<ActionOutputReady> {
    let slot = workflow
        .node(ticket.step)
        .and_then(|node| node.output.slot())
        .ok_or("retry action output slot missing")?;
    let value = SlotValue::I64(23);
    let encoded = postcard::to_allocvec(&value)?;
    Ok(ActionOutputReady {
        output_slot: slot,
        value,
        taint: Taint::Clean,
        encoded_len: u32::try_from(encoded.len()).map_err(|_| "action output length overflow")?,
    })
}

fn wait_for_timer(deadline: std::time::Instant) {
    std::thread::sleep(deadline.saturating_duration_since(std::time::Instant::now()));
}
