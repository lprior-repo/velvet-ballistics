use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, Idempotency,
    RetryPolicy, RetrySafety, SideEffect,
};

use vb_core::{ActionId, CapabilitySet, RunId, RuntimePolicy, SlotIdx, SlotValue, Taint};
use vb_storage::JournalEvent;

use super::harness::Scratch;
use super::support::{self, Durability, TestResult};

const RUN: RunId = RunId::new(0xA301);

#[test]
fn retry_timer_cold_recovery_preserves_identity_and_refuses_stale_ticket() -> TestResult {
    let source = "version: velvet-ballistics/v1\nname: ingress_active_retry\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: seed, value: '42'}\n  - id: work\n    try_again: {max_attempts: 2, delay: '2s'}\n    do: {action: '0', input: '$step.seed'}\n  - id: done\n    finish: {result: '$step.seed'}\n";
    let workflow = support::compile(source)?;
    let contract = action_contract()?;
    let scratch = Scratch::new("retry")?;
    let mut live =
        support::open_runtime(scratch.path(), RuntimePolicy::Journaled, Durability::Strict)?;
    support::persist_artifact(
        &live.journal,
        &workflow,
        RuntimePolicy::Journaled,
        std::slice::from_ref(&contract),
    )?;
    live.runtime
        .submit_direct_with_inputs_grants_and_contracts(
            RUN,
            workflow.clone(),
            Box::new([]),
            CapabilitySet::empty(),
            Box::from([contract.clone()]),
        )?;
    let do_step = super::harness::step(&workflow, "work")?;
    let mut first = None;
    for _ in 0..64 {
        live.runtime.tick_all()?;
        if let Some(ticket) = live
            .runtime
            .lookup_pending_action_ticket(RUN, u64::from(do_step.get()))
        {
            first = Some(ticket);
            break;
        }
    }
    let first = first.ok_or("first Do attempt did not become pending")?;
    assert_eq!(first.attempt, 1);
    live.runtime.fail_action(
        first.clone(),
        ActionFailure {
            code: ActionFailureCode::Timeout,
            retry_policy: RetryPolicy::Retryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        },
    )?;
    live.runtime.tick_all()?;
    let mut scheduled = None;
    for _ in 0..32 {
        let events = support::events(&live.journal, RUN)?;
        if events.iter().any(|event| {
            matches!(
                event,
                JournalEvent::RetryTimerScheduled { run, .. } if *run == RUN
            )
        }) {
            scheduled = Some(events);
            break;
        }
        live.runtime.tick_all()?;
    }
    let scheduled = scheduled.ok_or_else(|| {
        format!(
            "retry timer was not journaled; history: {:?}",
            support::events(&live.journal, RUN)
        )
    })?;
    assert_eq!(
        scheduled
            .iter()
            .filter(|event| matches!(
                event,
                JournalEvent::RetryTimerScheduled { run, .. } if *run == RUN
            ))
            .count(),
        1
    );
    assert!(
        live.runtime
            .lookup_pending_action_ticket(RUN, u64::from(do_step.get()))
            .is_none()
    );
    let timer = live.runtime.capture_timer_entry(RUN)?;
    let before_product = live.runtime.recover_product(RUN)?;
    drop(live);

    let mut reopened =
        support::open_runtime(scratch.path(), RuntimePolicy::Journaled, Durability::Strict)?;
    assert_eq!(reopened.runtime.recover_product(RUN)?, before_product);
    assert_eq!(support::events(&reopened.journal, RUN)?, scheduled);
    reopened.runtime.recover_and_resume(RUN)?;
    reopened.runtime.tick_all()?;
    let recovered_timer = reopened.runtime.capture_timer_entry(RUN)?;
    assert_eq!(recovered_timer.run, timer.run);
    assert_eq!(recovered_timer.generation, timer.generation);
    std::thread::sleep(
        recovered_timer
            .deadline
            .saturating_duration_since(std::time::Instant::now()),
    );
    reopened.runtime.timer_entry_fired(recovered_timer)?;
    reopened.runtime.tick_all()?;
    let second = wait_for_attempt(&mut reopened.runtime, RUN, do_step, 2)?;
    assert_ne!(second.attempt, first.attempt);
    let before_stale = support::events(&reopened.journal, RUN)?;
    let output_slot = workflow
        .node(do_step)
        .and_then(|node| node.output.slot())
        .ok_or("Do output slot missing")?;
    let stale_output = output(output_slot, SlotValue::I64(99))?;
    assert_eq!(
        reopened
            .runtime
            .complete_action_with_output(first, stale_output),
        Err(vb_runtime::RuntimeError::StaleAttempt {
            incoming: 1,
            current: 2,
        })
    );
    assert_eq!(
        support::events(&reopened.journal, RUN)?,
        before_stale,
        "stale attempt mutated durable history"
    );
    assert_eq!(
        reopened
            .runtime
            .lookup_pending_action_ticket(RUN, u64::from(do_step.get())),
        Some(second.clone())
    );

    reopened
        .runtime
        .complete_action_with_output(second, output(output_slot, SlotValue::I64(99))?)?;
    support::tick_until_terminal(&mut reopened.runtime, RUN, 128)?;
    let completed = support::events(&reopened.journal, RUN)?;
    assert_eq!(
        completed.get(..scheduled.len()),
        Some(scheduled.as_slice()),
        "cold recovery changed the ordered retry event prefix"
    );
    let result = support::terminal(&completed, RUN)?;
    assert_eq!(result.value, SlotValue::I64(42));
    assert_eq!(result.taint, Taint::Clean);
    assert_eq!(completed.iter().filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { run, ticket, .. } if *run == RUN && ticket.idempotency_key == second.idempotency_key && ticket.attempt == second.attempt)).count(), 1);
    Ok(())
}

fn wait_for_attempt(
    runtime: &mut vb_runtime::runtime::Runtime,
    run: RunId,
    step: vb_core::StepIdx,
    attempt: usize,
) -> TestResult<vb_core::action::ActionTicket> {
    for _ in 0..128 {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_action_ticket(run, u64::from(step.get())) {
            if ticket.attempt == u16::try_from(attempt)? {
                return Ok(ticket);
            }
        }
    }
    Err(format!(
        "Do attempt {attempt} did not become pending; ticket={:?}, timer={:?}, frame={:?}",
        runtime.lookup_pending_action_ticket(run, u64::from(step.get())),
        runtime.capture_timer_entry(run),
        runtime
            .inspect_run_state(run)
            .map(|state| state.frame.checkpoint_v1())
    )
    .into())
}

fn output(slot: SlotIdx, value: SlotValue) -> TestResult<ActionOutputReady> {
    Ok(ActionOutputReady {
        output_slot: slot,
        encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
        value,
        taint: Taint::Clean,
    })
}

fn action_contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("ingress-active-retry")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5_000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}
