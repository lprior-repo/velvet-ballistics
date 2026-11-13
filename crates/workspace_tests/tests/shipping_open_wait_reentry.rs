#![forbid(unsafe_code)]

#[path = "shipping_ingress_matrix/support.rs"]
mod support;

use vb_core::action::{
    ActionContract, ActionName, ActionOutputReady, Idempotency, RetrySafety, SideEffect,
};
use vb_core::value::Taint;
use vb_core::{ActionId, CompiledNodeKind, RunId, RuntimePolicy, SlotValue, StepState};
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};
use vb_storage::JournalEvent;

use support::{Durability, TestResult, compile, events, open_runtime, tick_until_action};

#[path = "shipping_open_wait_reentry/owned.rs"]
mod owned;

fn action_contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("open-wait-reentry-route")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5_000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([]),
    })
}

fn action_output(output_slot: vb_core::SlotIdx, value: i64) -> TestResult<ActionOutputReady> {
    let value = SlotValue::I64(value);
    Ok(ActionOutputReady {
        output_slot,
        encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
        value,
        taint: Taint::Clean,
    })
}

const SOURCE: &str = "version: velvet-ballistics/v1\nname: repeat_open_wait_reentry\nwhen: {manual: {}}\ninputs: {retained: {is: list, of: number, max: 1}}\nsteps:\n  - id: seed\n    set: {output: seed, value: '42'}\n  - id: event_id\n    set: {output: event_id, value: '1'}\n  - id: retry_loop\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: route_action\n          do: {action: '0', input: '$step.seed'}\n        - id: route\n          choose:\n            branches:\n              - when: '$step.route_action == 1'\n                steps:\n                  - id: gate\n                    wait: {event: '$step.event_id'}\n            otherwise: skip_wait\n        - id: skip_wait\n          set: {output: skip, value: 'true'}\n  - id: done\n    finish: {result: '$step.seed'}\n";

#[test]
fn repeat_reentry_open_wait_survives_fjall_cold_recovery() -> TestResult {
    let workflow = compile(SOURCE)?;
    let wait_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::WaitEvent { .. }))
        .map(|node| node.id)
        .ok_or("accepted workflow has no WaitEvent marker")?;
    let repeat_check_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::RepeatCheck { .. }))
        .map(|node| node.id)
        .ok_or("accepted workflow has no RepeatCheck")?;
    let route_action_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .map(|node| node.id)
        .ok_or("accepted workflow has no route action")?;
    let route_output = workflow
        .node(route_action_step)
        .and_then(|node| node.output.slot())
        .ok_or("route action output slot missing")?;
    let contracts: Box<[ActionContract]> = Box::from([action_contract()?]);
    let dir = tempfile::tempdir()?;
    let run = RunId::new(1_047);
    let baseline_events;
    let max_started_step;
    let frame_at_wait;
    let store_at_wait;
    let retained_input;
    {
        let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
        retained_input =
            owned::commit_owned_input(&open.journal, &workflow, SOURCE, run, contracts.as_ref())?;
        open.runtime
            .activate_owned_submission(run, retained_input.source_digest)?;
        let first_ticket = tick_until_action(
            &mut open.runtime,
            run,
            u64::from(route_action_step.get()),
            64,
        )?;
        open.runtime
            .complete_action_with_output(first_ticket, action_output(route_output, 0)?)?;
        let second_ticket = tick_until_action(
            &mut open.runtime,
            run,
            u64::from(route_action_step.get()),
            64,
        )?;
        assert_ne!(first_ticket, second_ticket);
        open.runtime
            .complete_action_with_output(second_ticket, action_output(route_output, 1)?)?;
        for _ in 0..64 {
            open.runtime.tick_all()?;
            if open
                .runtime
                .inspect_run_state(run)?
                .frame
                .step_state(wait_step)?
                == StepState::Waiting
            {
                break;
            }
        }
        let state = open.runtime.inspect_run_state(run)?;
        assert_eq!(state.frame.pc(), wait_step);
        assert_eq!(state.frame.step_state(wait_step)?, StepState::Waiting);
        assert_eq!(
            state.frame.read_slot(retained_input.slot)?,
            &SlotValue::List(retained_input.list)
        );
        assert_eq!(
            state.store.list(retained_input.list)?,
            &[SlotValue::I64(73)]
        );
        frame_at_wait = state.frame.checkpoint_v1();
        store_at_wait = state.store.checkpoint_v1();
        baseline_events = events(&open.journal, run)?;
        max_started_step = baseline_events
            .iter()
            .filter_map(|event| match event {
                JournalEvent::StepStarted { step, .. } => Some(*step),
                _ => None,
            })
            .max()
            .ok_or("run has no durable StepStarted event")?;
        assert!(
            wait_step < max_started_step,
            "reentry must suspend below historical maximum"
        );
        assert_eq!(
            baseline_events
                .iter()
                .filter(|event| matches!(
                    event,
                    JournalEvent::StepStarted { step, .. } if *step == route_action_step
                ))
                .count(),
            2,
            "route action must execute once per repeat iteration"
        );
        assert_eq!(
            baseline_events
                .iter()
                .filter(|event| matches!(
                    event,
                    JournalEvent::ActionScheduledTicket { ticket, .. }
                        if ticket.step == route_action_step
                ))
                .count(),
            2
        );
        assert!(
            baseline_events.iter().any(|event| matches!(
                event,
                JournalEvent::StepSucceeded { step, .. } if *step == repeat_check_step
            )),
            "first iteration must durably complete the higher RepeatCheck"
        );
        let wait_started_attempt = baseline_events.iter().find_map(|event| match event {
            JournalEvent::StepStarted { step, attempt, .. } if *step == wait_step => Some(*attempt),
            _ => None,
        });
        assert_eq!(
            wait_started_attempt,
            Some(1),
            "StepStarted identity for open wait marker {wait_step:?}"
        );
    }

    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    assert_eq!(events(&open.journal, run)?, baseline_events);
    let product = open.runtime.recover_product(run)?;
    match &product {
        RuntimeRecoveryProduct::Resumable(product) => assert_eq!(
            product.boundary_kind(),
            RecoveredRunBoundaryKind::OpenWaitEvent
        ),
        other => return Err(format!("Repeat reentry recovered as {other:?}").into()),
    }
    open.runtime.recover_and_resume(run)?;
    open.runtime.tick_all()?;
    let state = open.runtime.inspect_run_state(run)?;
    assert_eq!(state.frame.pc(), wait_step);
    assert_eq!(state.frame.step_state(wait_step)?, StepState::Waiting);
    assert_eq!(state.frame.read_slot(route_output)?, &SlotValue::I64(1));
    assert_eq!(state.frame.read_taint(route_output)?, Taint::Clean);
    assert_eq!(state.frame.checkpoint_v1(), frame_at_wait);
    assert_eq!(state.store.checkpoint_v1(), store_at_wait);
    let SlotValue::List(retained_list) = state.frame.read_slot(retained_input.slot)? else {
        return Err("restored retained input is not a list handle".into());
    };
    assert_eq!(*retained_list, retained_input.list);
    assert_eq!(state.store.list(*retained_list)?, &[SlotValue::I64(73)]);
    let counters = open.runtime.counters_snapshot();
    assert_eq!(counters.runs_submitted, 0);
    assert_eq!(counters.runs_completed, 0);
    assert_eq!(counters.runs_failed, 0);
    assert_eq!(counters.steps_executed, 0);
    assert_eq!(events(&open.journal, run)?, baseline_events);

    open.runtime.tick_all()?;
    let state = open.runtime.inspect_run_state(run)?;
    assert_eq!(state.frame.pc(), wait_step);
    assert_eq!(state.frame.step_state(wait_step)?, StepState::Waiting);
    assert_eq!(events(&open.journal, run)?, baseline_events);

    let corruption_slot = baseline_events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::SlotWrittenEvent { slot, .. } => Some(*slot),
            _ => None,
        })
        .ok_or("run has no durable slot write to corrupt")?;
    let corrupted_dir = tempfile::tempdir()?;
    let corrupted = open_runtime(
        corrupted_dir.path(),
        RuntimePolicy::Strict,
        Durability::Strict,
    )?;
    owned::commit_owned_input(
        &corrupted.journal,
        &workflow,
        SOURCE,
        run,
        &[action_contract()?],
    )?;
    let prefix = events(&corrupted.journal, run)?;
    let prefix_len = prefix.len();
    assert_eq!(
        baseline_events
            .iter()
            .take(prefix_len)
            .cloned()
            .collect::<Vec<_>>(),
        prefix
    );
    let (corrupt_events, raw_before) = owned::append_corrupt_slot_taint(
        &corrupted.journal,
        run,
        &baseline_events,
        prefix_len,
        corruption_slot,
    )?;
    assert_eq!(
        owned::slot_written_taint(&baseline_events, corruption_slot)?,
        Taint::Clean
    );
    assert_eq!(
        owned::slot_written_taint(&corrupt_events, corruption_slot)?,
        Taint::Secret
    );
    drop(corrupted.runtime);
    drop(corrupted.journal);
    drop(open.runtime);
    drop(open.journal);

    let cold = open_runtime(
        corrupted_dir.path(),
        RuntimePolicy::Strict,
        Durability::Strict,
    )?;
    assert!(cold.runtime.list_active_runs(64, None).is_empty());
    assert_eq!(
        cold.runtime
            .lookup_pending_action_ticket(run, u64::from(wait_step.get())),
        None
    );
    match cold.runtime.recover_product(run) {
        Err(vb_runtime::RuntimeError::Recovery { error }) => assert_eq!(
            error,
            "replay divergence at step StepIdx(8): capture differs from artifact replay"
        ),
        other => return Err(format!("corrupt slot taint result: {other:?}").into()),
    }
    assert!(cold.runtime.list_active_runs(64, None).is_empty());
    assert_eq!(
        cold.runtime
            .lookup_pending_action_ticket(run, u64::from(wait_step.get())),
        None
    );
    assert_eq!(events(&cold.journal, run)?, corrupt_events);
    let raw_after = owned::raw_event_bytes(&cold.journal, run, &corrupt_events)?;
    assert_eq!(raw_after, raw_before);
    Ok(())
}
