use std::{num::NonZeroUsize, sync::Arc, time::Instant};
use vb_core::{CompiledNodeKind as K, RunId, RuntimePolicy, SlotValue, Taint};
use vb_runtime::{
    journal::{RuntimeJournalEvent as E, VolatileRuntimeJournal},
    runtime::Runtime,
    shard::{AskAnswer, ShardConfig},
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn exercise(
    repeat: bool,
    ask: bool,
    named: bool,
    quantum: u64,
) -> TestResult<Vec<(vb_core::StepIdx, vb_core::SlotIdx)>> {
    let suspension = if ask {
        "ask: {prompt: '$seed'}"
    } else {
        "wait: {timeout: '$delay'}"
    };
    let reference = if named { "$steps.response" } else { "$seed" };
    let leaf = format!(
        "- id: response\n  {suspension}\n- id: observed\n  set: {{output: observed, value: '{reference}'}}\n"
    );
    let indent = |body: &str, spaces: usize| {
        body.lines()
            .map(|line| format!("{}{line}\n", " ".repeat(spaces)))
            .collect::<String>()
    };
    let branch = format!(
        "- id: branch\n  choose:\n    branches:\n      - when: 'true'\n        steps:\n{}",
        indent(&leaf, 10)
    );
    let body = if repeat {
        format!(
            "- id: bounded\n  repeat:\n    max_attempts: 2\n    steps:\n{}",
            indent(
                &(branch.clone() + "- id: tail\n  set: {output: tail, value: '9'}\n"),
                6
            )
        )
    } else {
        branch
    };
    let yaml = format!(
        "version: velvet-ballistics/v1\nname: nested_suspension\nwhen: {{manual: {{}}}}\nsteps:\n  - id: seed\n    set: {{output: saved, value: '42'}}\n  - id: delay\n    set: {{output: delay, value: '10'}}\n{}  - id: done\n    finish: {{result: '$seed'}}\n",
        indent(&body, 2)
    );
    let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
    vb_validate::shared::validate(&plan.to_parts())?;
    let suspension_node = plan
        .to_parts()
        .nodes
        .into_iter()
        .find(|node| matches!(node.kind, K::Ask { .. } | K::WaitUntil { .. }))
        .ok_or("suspension node")?;
    let named_output = if named {
        let resume = plan
            .node(suspension_node.routing.next.step().ok_or("resume")?)
            .ok_or("resume node")?;
        Some(
            plan.node(resume.routing.next.step().ok_or("consumer")?)
                .and_then(|node| node.output.slot())
                .ok_or("named Ask consumer output")?,
        )
    } else {
        None
    };
    let run = RunId::new(1);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Relaxed,
            step_budget_per_tick: quantum,
            ..ShardConfig::default()
        },
        journal.clone(),
    );
    runtime.submit_direct(run, plan)?;
    let expected = if repeat { 2 } else { 1 };
    let mut boundaries = 0;
    let mut tickets = Vec::new();
    let mut answers = Vec::new();
    let mut timers = Vec::new();
    for _ in 0..96 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
        assert_eq!(
            runtime.counters_snapshot().runs_failed,
            0,
            "{:?}",
            journal.snapshot()?
        );
        if ask {
            if let Some(ticket) = runtime.lookup_pending_ask_ticket(run, suspension_node.id) {
                assert!(
                    !tickets.contains(&ticket),
                    "fresh Ask authority for each iteration"
                );
                assert_eq!(ticket.ask_step(), suspension_node.id);
                let value = SlotValue::I64(
                    73_i64
                        .checked_add(i64::try_from(boundaries)?)
                        .ok_or("answer overflow")?,
                );
                let taint = if boundaries == 0 {
                    Taint::DerivedFromSecret
                } else {
                    Taint::Clean
                };
                runtime.answer_ask(AskAnswer::new(ticket, ticket.answer_slot(), value, taint))?;
                answers.push((value, taint));
                tickets.push(ticket);
                boundaries += 1;
            } else {
                let state = runtime.inspect_run_state(run)?;
                if state.frame.step_state(state.frame.pc())? == vb_core::StepState::Pending {
                    runtime.resume_run(run)?;
                }
            }
        } else if let Ok(timer) = runtime.capture_timer_entry(run) {
            assert!(
                !timers.contains(&timer),
                "fresh Wait authority for each iteration"
            );
            assert_eq!(timer.kind, vb_runtime::shard::PendingTimerKind::Wait);
            std::thread::sleep(timer.deadline.saturating_duration_since(Instant::now()));
            runtime.timer_entry_fired(timer)?;
            timers.push(timer);
            boundaries += 1;
        } else {
            let state = runtime.inspect_run_state(run)?;
            if state.frame.step_state(state.frame.pc())? == vb_core::StepState::Pending {
                runtime.resume_run(run)?;
            }
        }
    }
    assert_eq!(boundaries, expected);
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(
        runtime.counters_snapshot().runs_failed,
        0,
        "{:?}",
        journal.snapshot()?
    );
    let events = journal.snapshot()?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::AskScheduled { .. } if ask)
                || matches!(event, E::WaitScheduled { .. } if !ask))
            .count(),
        expected
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::AskAnswered { .. } if ask)
                || matches!(event, E::WaitResolved { .. } if !ask))
            .count(),
        expected
    );
    if ask {
        let answer = suspension_node.output.slot().ok_or("answer slot")?;
        let writes = events
            .iter()
            .filter_map(|event| match event {
                E::SlotWritten {
                    slot, value, taint, ..
                } if *slot == answer => Some((value, taint)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            writes.len(),
            expected.checked_mul(2).ok_or("write count overflow")?
        );
        let actual = writes
            .into_iter()
            .map(|(value, taint)| {
                postcard::from_bytes::<SlotValue>(value).map(|value| (value, *taint))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let expected_writes = answers
            .iter()
            .flat_map(|answer| [*answer, *answer])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected_writes);
    }
    if let Some(output) = named_output {
        let writes = events
            .iter()
            .filter_map(|event| match event {
                E::SlotWritten {
                    slot, value, taint, ..
                } if *slot == output => Some((value, taint)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(writes.len(), expected);
        let actual = writes
            .into_iter()
            .map(|(value, taint)| {
                postcard::from_bytes::<SlotValue>(value).map(|value| (value, *taint))
            })
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(actual, answers);
    }
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::RunFinished { .. }))
            .count(),
        1
    );
    Ok(events
        .iter()
        .filter_map(|event| match event {
            E::StepSucceeded { step, output, .. } => Some((*step, *output)),
            _ => None,
        })
        .collect())
}

fn parity(repeat: bool, ask: bool, named: bool) -> TestResult {
    assert_eq!(
        exercise(repeat, ask, named, 1)?,
        exercise(repeat, ask, named, 64)?
    );
    Ok(())
}

#[test]
fn repeat_choose_wait_uses_runtime_timer_authority_each_iteration() -> TestResult {
    parity(true, false, false)
}
#[test]
fn choose_wait_uses_runtime_timer_authority() -> TestResult {
    parity(false, false, false)
}
#[test]
fn repeat_choose_ask_uses_runtime_tickets_each_iteration() -> TestResult {
    parity(true, true, false)
}
#[test]
fn choose_ask_uses_runtime_ticket() -> TestResult {
    parity(false, true, false)
}
#[test]
fn repeat_choose_ask_named_result_preserves_answer_and_taint() -> TestResult {
    parity(true, true, true)
}
#[test]
fn choose_ask_named_result_preserves_answer_and_taint() -> TestResult {
    parity(false, true, true)
}
