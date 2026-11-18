use std::{num::NonZeroUsize, sync::Arc};
use vb_core::action::{
    ActionContract, ActionName, ActionOutputReady, Idempotency, RetrySafety, SideEffect,
};
use vb_core::{
    ActionId, CapabilitySet, CompiledNodeKind as K, RunId, RuntimePolicy, SlotValue, Taint,
};
use vb_runtime::{
    journal::{RuntimeJournalEvent as E, VolatileRuntimeJournal},
    runtime::Runtime,
    shard::{AskAnswer, ShardConfig},
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn runtime(quantum: u64) -> (Runtime, Arc<VolatileRuntimeJournal>) {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    (
        Runtime::new(
            NonZeroUsize::MIN,
            ShardConfig {
                policy: RuntimePolicy::Relaxed,
                step_budget_per_tick: quantum,
                ..ShardConfig::default()
            },
            journal.clone(),
        ),
        journal,
    )
}

#[test]
fn idle_slices_reach_real_ask_without_reissuing_pending_authority() -> TestResult {
    for quantum in [1, 64] {
        let plan = vb_compile::compile_workflow(
            br#"version: velvet-ballistics/v1
name: slice_then_ask
when: {manual: {}}
steps:
  - id: seed
    set: {output: saved, value: '42'}
  - id: question
    ask: {prompt: '$seed'}
  - id: done
    finish: {result: '$seed'}
"#,
        )?;
        let ask = plan
            .to_parts()
            .nodes
            .into_iter()
            .find(|node| matches!(node.kind, K::Ask { .. }))
            .ok_or("Ask")?
            .id;
        let run = RunId::new(1);
        let (mut runtime, journal) = runtime(quantum);
        runtime.submit_direct(run, plan)?;
        let mut ticket = None;
        for _ in 0..8 {
            runtime.tick_all()?;
            ticket = runtime.lookup_pending_ask_ticket(run, ask);
            if ticket.is_some() {
                break;
            }
        }
        let ticket = ticket.ok_or("one-step slice stranded the pending Ask")?;
        let before = runtime.inspect_run_state(run)?.clone();
        let events = journal.snapshot()?;
        for _ in 0..4 {
            runtime.tick_all()?;
        }
        assert_eq!(runtime.lookup_pending_ask_ticket(run, ask), Some(ticket));
        assert_eq!(runtime.inspect_run_state(run)?, &before);
        assert_eq!(journal.snapshot()?, events);
        runtime.answer_ask(AskAnswer::new(
            ticket,
            ticket.answer_slot(),
            SlotValue::I64(73),
            Taint::Clean,
        ))?;
        for _ in 0..8 {
            runtime.tick_all()?;
            if runtime.counters_snapshot().runs_completed == 1 {
                break;
            }
        }
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    }
    Ok(())
}

#[test]
fn pending_runs_rotate_across_idle_and_queued_resume_ticks() -> TestResult {
    let plan = vb_compile::compile_workflow(
        br#"version: velvet-ballistics/v1
name: queued_slice_fairness
when: {manual: {}}
steps:
  - id: seed
    set: {output: saved, value: '42'}
  - id: bounded
    repeat:
      max_attempts: 8
      steps:
        - id: leaf
          set: {output: leaf, value: '9'}
  - id: done
    finish: {result: '$seed'}
"#,
    )?;
    let runs = [RunId::new(1), RunId::new(2), RunId::new(3)];
    let (mut runtime, _) = runtime(1);
    for run in runs {
        runtime.submit_direct(run, plan.clone())?;
    }
    for _ in runs {
        runtime.tick_all()?;
    }
    for run in runs {
        assert_eq!(runtime.inspect_run_state(run)?.frame.executed(), 1);
    }
    for _ in runs {
        runtime.tick_all()?;
    }
    for run in runs {
        assert_eq!(runtime.inspect_run_state(run)?.frame.executed(), 2);
    }
    for _ in runs {
        runtime.resume_run(RunId::new(1))?;
        runtime.tick_all()?;
    }
    for run in runs {
        assert_eq!(runtime.inspect_run_state(run)?.frame.executed(), 3);
    }
    for _ in 0..128 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_completed == 3 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 3);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    Ok(())
}

#[test]
fn idle_and_resume_ticks_preserve_real_pending_do_ticket_and_frame() -> TestResult {
    let plan = vb_compile::compile_workflow(
        br#"version: velvet-ballistics/v1
name: pending_action_slice_guard
when: {manual: {}}
steps:
  - id: seed
    set: {output: saved, value: '42'}
  - id: work
    do: {action: '0', input: '$seed'}
  - id: done
    finish: {result: '$seed'}
"#,
    )?;
    let action = plan
        .to_parts()
        .nodes
        .into_iter()
        .find(|node| matches!(node.kind, K::Do { .. }))
        .ok_or("Do")?;
    let output = action.output.slot().ok_or("output")?;
    let run = RunId::new(1);
    let (mut runtime, journal) = runtime(1);
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        plan,
        Box::new([]),
        CapabilitySet::empty(),
        Box::from([ActionContract {
            id: ActionId::new(0),
            name: ActionName::new("slice-action")?,
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
    let mut ticket = None;
    for _ in 0..8 {
        runtime.tick_all()?;
        ticket = runtime.lookup_pending_action_ticket(run, u64::from(action.id.get()));
        if ticket.is_some() {
            break;
        }
    }
    let ticket = ticket.ok_or("real Do ticket")?;
    let before = runtime.inspect_run_state(run)?.frame.clone();
    let events = journal.snapshot()?;
    for _ in 0..4 {
        runtime.tick_all()?;
    }
    assert_eq!(journal.snapshot()?, events);
    runtime.resume_run(run)?;
    runtime.tick_all()?;
    assert_eq!(
        runtime.lookup_pending_action_ticket(run, u64::from(action.id.get())),
        Some(ticket)
    );
    assert_eq!(&runtime.inspect_run_state(run)?.frame, &before);
    let events = journal.snapshot()?;
    let scheduled = events
        .iter()
        .filter_map(|event| match event {
            E::ActionScheduledTicket { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(scheduled, vec![ticket]);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::StepStarted { step, .. } if *step == action.id))
            .count(),
        1
    );
    let value = SlotValue::I64(73);
    runtime.complete_action_with_output(
        ticket,
        ActionOutputReady {
            output_slot: output,
            value,
            encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
            taint: Taint::Clean,
        },
    )?;
    for _ in 0..8 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    Ok(())
}
