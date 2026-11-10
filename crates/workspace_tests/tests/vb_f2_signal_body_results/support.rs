use std::{num::NonZeroUsize, sync::Arc, time::Instant};
use vb_core::{CompiledNodeKind as K, RunId, RuntimePolicy, SlotValue, Taint};
use vb_runtime::{
    journal::{RuntimeJournalEvent as E, VolatileRuntimeJournal},
    runtime::Runtime,
    shard::{AskAnswer, ShardConfig},
};
pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) struct Outcome {
    pub(super) value: SlotValue,
    pub(super) events: Vec<E>,
    pub(super) asks: usize,
    pub(super) waits: usize,
}
pub(super) fn indent(body: &str, n: usize) -> String {
    body.lines()
        .map(|line| format!("{}{line}\n", " ".repeat(n)))
        .collect()
}
pub(super) fn source(declarations: &str, body: &str, result: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: signal_body_results\nwhen: {{manual: {{}}}}\n{declarations}steps:\n  - id: seed\n    set: {{output: saved, value: '42'}}\n  - id: delay\n    set: {{output: delay, value: '10'}}\n{}  - id: done\n    finish: {{result: '{result}'}}\n",
        indent(body, 2)
    )
}
pub(super) fn execute(
    source: &str,
    quantum: u64,
    dynamic: Option<SlotValue>,
) -> TestResult<Outcome> {
    let plan = vb_compile::compile_workflow(source.as_bytes())?;
    vb_validate::shared::validate(&plan.to_parts())?;
    let ask_steps = plan
        .to_parts()
        .nodes
        .iter()
        .filter_map(|node| matches!(node.kind, K::Ask { .. }).then_some(node.id))
        .collect::<Vec<_>>();
    let inputs = match dynamic {
        Some(value) => vec![(plan.input_slots().first().ok_or("input")?.slot, value)],
        None => Vec::new(),
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
    runtime.submit_compiled_with_inputs(run, plan, inputs.into_boxed_slice())?;
    let mut asks = 0_usize;
    let mut waits = 0_usize;
    let mut tickets = Vec::new();
    for _ in 0..512 {
        runtime.tick_all()?;
        assert_eq!(
            runtime.counters_snapshot().runs_failed,
            0,
            "{:?}",
            journal.snapshot()?
        );
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
        let mut answered = false;
        for &step in &ask_steps {
            if let Some(ticket) = runtime.lookup_pending_ask_ticket(run, step) {
                assert!(!tickets.contains(&ticket));
                let answer =
                    SlotValue::I64(73_i64.checked_add(i64::try_from(asks)?).ok_or("answer")?);
                runtime.answer_ask(AskAnswer::new(
                    ticket,
                    ticket.answer_slot(),
                    answer,
                    if asks == 0 {
                        Taint::DerivedFromSecret
                    } else {
                        Taint::Clean
                    },
                ))?;
                tickets.push(ticket);
                asks = asks.checked_add(1).ok_or("asks")?;
                answered = true;
            }
        }
        if !answered && let Ok(timer) = runtime.capture_timer_entry(run) {
            assert_eq!(timer.kind, vb_runtime::shard::PendingTimerKind::Wait);
            std::thread::sleep(timer.deadline.saturating_duration_since(Instant::now()));
            runtime.timer_entry_fired(timer)?;
            waits = waits.checked_add(1).ok_or("waits")?;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = journal.snapshot()?;
    let result = events
        .iter()
        .filter_map(|event| match event {
            E::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(result.len(), 1);
    let result = *result.first().ok_or("finish")?;
    let value = events
        .iter()
        .rev()
        .find_map(|event| match event {
            E::SlotWritten { slot, value, .. } if *slot == result => Some(value),
            _ => None,
        })
        .ok_or("result write")?;
    let value = postcard::from_bytes(value)?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::AskScheduled { .. }))
            .count(),
        asks
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::AskAnswered { .. }))
            .count(),
        asks
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::WaitScheduled { .. }))
            .count(),
        waits
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::WaitResolved { .. }))
            .count(),
        waits
    );
    Ok(Outcome {
        value,
        events,
        asks,
        waits,
    })
}
