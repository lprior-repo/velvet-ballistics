#![forbid(unsafe_code)]

use std::{num::NonZeroUsize, sync::Arc};
use vb_compile::compile_workflow;
use vb_core::action::{
    ActionContract, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket, Idempotency,
    RetrySafety, SideEffect,
};
use vb_core::{
    ActionId, CapabilitySet, CompiledNodeKind, CompiledWorkflow, RunId, SlotIdx, SlotValue, Taint,
};
use vb_runtime::{
    journal::{RuntimeJournalEvent, VolatileRuntimeJournal},
    runtime::Runtime,
    shard::ShardConfig,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn runtime() -> (Runtime, Arc<VolatileRuntimeJournal>) {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: vb_core::RuntimePolicy::Relaxed,
            ..ShardConfig::default()
        },
        journal.clone(),
    );
    (runtime, journal)
}

fn submit(runtime: &mut Runtime, run: RunId, workflow: CompiledWorkflow) -> TestResult {
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::new([]),
        CapabilitySet::empty(),
        Box::from([ActionContract {
            id: ActionId::new(0),
            name: ActionName::new("observable-action")?,
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        }]),
    )?;
    runtime.tick_all()?;
    Ok(())
}

fn complete(
    runtime: &mut Runtime,
    ticket: ActionTicket,
    output: SlotIdx,
    value: SlotValue,
    taint: Taint,
) -> TestResult {
    runtime.complete_action_with_output(
        ticket,
        ActionOutputReady {
            output_slot: output,
            value,
            encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
            taint,
        },
    )?;
    runtime.tick_all()?;
    Ok(())
}

fn writes(
    events: &[RuntimeJournalEvent],
    run: RunId,
    wanted: SlotIdx,
) -> TestResult<Vec<(SlotValue, Taint)>> {
    events
        .iter()
        .filter_map(|event| match event {
            RuntimeJournalEvent::SlotWritten {
                run: actual,
                slot,
                value,
                taint,
                ..
            } if *actual == run && *slot == wanted => Some(
                postcard::from_bytes(value)
                    .map(|value| (value, *taint))
                    .map_err(Into::into),
            ),
            _ => None,
        })
        .collect()
}

fn finishes(events: &[RuntimeJournalEvent], run: RunId) -> Vec<SlotIdx> {
    events
        .iter()
        .filter_map(|event| match event {
            RuntimeJournalEvent::RunFinished {
                run: actual,
                result,
                ..
            } if *actual == run => Some(*result),
            _ => None,
        })
        .collect()
}

fn completions(
    events: &[RuntimeJournalEvent],
    run: RunId,
    wanted: SlotIdx,
) -> TestResult<Vec<(SlotValue, Taint)>> {
    events
        .iter()
        .filter_map(|event| match event {
            RuntimeJournalEvent::ActionCompletedEnvelope {
                ticket,
                output,
                value,
                taint,
                ..
            } if ticket.run == run && *output == wanted => Some(
                postcard::from_bytes(value)
                    .map(|value| (value, *taint))
                    .map_err(Into::into),
            ),
            _ => None,
        })
        .collect()
}

#[test]
fn guards_dispatch_exact_action_tickets_for_scalar_and_repeat_bodies() -> TestResult {
    for condition in ["true", "false"] {
        for nested in [false, true] {
            let body = if nested {
                format!(
                    "  - id: group\n    if: '{condition}'\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: local\n          set: {{output: local, value: '7'}}\n        - id: work\n          do: {{action: '0', input: '$local'}}\n"
                )
            } else {
                format!(
                    "  - id: work\n    if: '{condition}'\n    do: {{action: '0', input: '$seed'}}\n"
                )
            };
            let source = format!(
                "version: velvet-ballistics/v1\nname: observable_guard\nwhen: {{manual: {{}}}}\nsteps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n{body}  - id: done\n    finish: {{result: seed}}\n"
            );
            let workflow = compile_workflow(source.as_bytes())?;
            let parts = workflow.to_parts();
            let action = parts
                .nodes
                .iter()
                .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
                .ok_or("action missing")?;
            let CompiledNodeKind::Do { input, .. } = action.kind else {
                return Err("expected action".into());
            };
            let output = action.output.slot().ok_or("action output missing")?;
            let (mut runtime, journal) = runtime();
            let run = RunId::new(71);
            submit(&mut runtime, run, workflow)?;
            let expected_count = match (condition, nested) {
                ("true", true) => 2,
                ("true", false) => 1,
                _ => 0,
            };
            let mut completed = Vec::new();
            for _ in 0..expected_count {
                let ticket = runtime
                    .lookup_pending_action_ticket(run, u64::from(action.id.get()))
                    .ok_or("guarded action ticket missing")?;
                assert_eq!(ticket.run, run);
                assert_eq!(ticket.step, action.id);
                assert_eq!(ticket.action, ActionId::new(0));
                assert_eq!(
                    *runtime.inspect_run_state(run)?.frame.read_slot(input)?,
                    SlotValue::I64(if nested { 7 } else { 42 })
                );
                completed.push(ticket);
                complete(
                    &mut runtime,
                    ticket,
                    output,
                    SlotValue::I64(99),
                    Taint::Clean,
                )?;
            }
            assert_eq!(
                runtime.lookup_pending_action_ticket(run, u64::from(action.id.get())),
                None
            );
            let events = journal.snapshot()?;
            let scheduled = events
                .iter()
                .filter_map(|event| match event {
                    RuntimeJournalEvent::ActionScheduledTicket {
                        ticket,
                        input: actual_input,
                        output: actual_output,
                        ..
                    } if ticket.run == run => {
                        assert_eq!(ticket.action, ActionId::new(0));
                        assert_eq!(*actual_input, input);
                        assert_eq!(*actual_output, output);
                        Some(*ticket)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(scheduled, completed);
            assert_eq!(
                completions(&events, run, output)?,
                vec![(SlotValue::I64(99), Taint::Clean); expected_count]
            );
            if condition == "true" {
                assert_eq!(writes(&events, run, output)?, Vec::new());
                if nested {
                    let result = parts
                        .nodes
                        .iter()
                        .find_map(|node| match node.kind {
                            CompiledNodeKind::RepeatFinish { result } => Some(result),
                            _ => None,
                        })
                        .ok_or("repeat result")?;
                    assert_ne!(result, output);
                    assert_eq!(
                        writes(&events, run, result)?,
                        vec![
                            (SlotValue::I64(99), Taint::Clean);
                            expected_count.checked_add(1).ok_or("write count")?
                        ]
                    );
                }
            }
            assert_eq!(finishes(&events, run), vec![SlotIdx::new(0)]);
            assert_eq!(runtime.counters_snapshot().runs_failed, 0);
            let before = events;
            runtime.tick_all()?;
            assert_eq!(journal.snapshot()?, before);
        }
    }
    Ok(())
}

#[test]
fn successful_retry_consumes_actual_result_and_preserves_operand_taint() -> TestResult {
    for taint in [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
        for failures in [1u16, 2] {
            let source = "version: velvet-ballistics/v1\nname: observable_retry\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: seed, value: '42'}\n  - id: work\n    try_again: {max_attempts: 3, delay: '0ms'}\n    do: {action: '0', input: '$seed'}\n  - id: observed\n    set: {output: observed, value: '$work + 1'}\n  - id: done\n    finish: {result: observed}\n";
            let (mut runtime, journal) = runtime();
            let run = RunId::new(123);
            submit(&mut runtime, run, compile_workflow(source.as_bytes())?)?;
            let first = runtime
                .lookup_pending_action_ticket(run, 1)
                .ok_or("initial ticket missing")?;
            let mut scheduled = Vec::new();
            for attempt in 1..=failures {
                let ticket = runtime
                    .lookup_pending_action_ticket(run, 1)
                    .ok_or("attempt ticket missing")?;
                assert_eq!(ticket.attempt, attempt);
                assert_eq!(ticket.capacity, 3);
                assert_eq!(ticket.idempotency_key, first.idempotency_key);
                scheduled.push(ticket);
                runtime.fail_action(ticket, ActionFailureCode::Timeout.into())?;
                runtime.tick_all()?;
            }
            let success = runtime
                .lookup_pending_action_ticket(run, 1)
                .ok_or("successful retry ticket missing")?;
            assert_eq!(
                success.attempt,
                failures.checked_add(1).ok_or("attempt overflow")?
            );
            assert_eq!(success.capacity, 3);
            assert_eq!(success.idempotency_key, first.idempotency_key);
            scheduled.push(success);
            complete(
                &mut runtime,
                success,
                SlotIdx::new(1),
                SlotValue::I64(99),
                taint,
            )?;
            let events = journal.snapshot()?;
            let tickets = events
                .iter()
                .filter_map(|event| match event {
                    RuntimeJournalEvent::ActionScheduledTicket { ticket, .. }
                        if ticket.run == run =>
                    {
                        Some(*ticket)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(tickets, scheduled);
            assert_eq!(
                completions(&events, run, SlotIdx::new(1))?,
                vec![(SlotValue::I64(99), taint)]
            );
            assert_eq!(
                writes(&events, run, SlotIdx::new(2))?,
                vec![(SlotValue::I64(100), taint)]
            );
            assert_eq!(finishes(&events, run), vec![SlotIdx::new(2)]);
            assert_eq!(runtime.lookup_pending_action_ticket(run, 1), None);
            assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        }
    }
    Ok(())
}
