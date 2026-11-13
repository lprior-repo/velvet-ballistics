use super::{concurrent_ir, drive_ir, two_lane_nodes};
use vb_core::engine::StepBudget;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::replay::step::{ReplayAction, ReplayCollectStates, replay_step_with_collect};
use vb_core::value::{ConstValue, SlotValue};
use vb_core::value_store::ValueStore;
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

fn plan(first_success: bool) -> Result<vb_core::CompiledWorkflow, String> {
    concurrent_ir(
        if first_success { "resume_first_success" } else { "resume_together" },
        two_lane_nodes(first_success),
        vec![ConstValue::I64(10), ConstValue::I64(20)],
        3,
    )
}

fn run_budget_one(
    first_success: bool,
) -> Result<(RuntimeSignal, Vec<EvidenceEvent>, Vec<SlotValue>), String> {
    let plan = plan(first_success)?;
    let mut run = RunFrame::new(
        RunId::new(1),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )
    .map_err(|error| format!("{error}"))?;
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let granted = vb_core::capability::CapabilitySet::empty();
    let mut all_events = Vec::new();
    for _ in 0..100 {
        let mut budget = StepBudget::new(1);
        let signal = drive_deterministic_full(
            &plan,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &granted,
            1,
        )
        .map_err(|error| format!("{error:?}"))?;
        all_events.extend(evidence.drain());
        if signal != RuntimeSignal::StepBudgetExhausted {
            let values = match signal {
                RuntimeSignal::Finished(SlotValue::List(id)) => store
                    .list(id)
                    .map_err(|error| format!("{error:?}"))?
                    .to_vec(),
                _ => Vec::new(),
            };
            return Ok((signal, all_events, values));
        }
    }
    Err("budget-one group did not finish within 100 drives".into())
}

fn replay_result(first_success: bool) -> Result<(SlotValue, Vec<SlotValue>), String> {
    let plan = plan(first_success)?;
    let mut run = RunFrame::new(
        RunId::new(2),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )
    .map_err(|error| format!("{error}"))?;
    let mut store = ValueStore::new();
    let mut states = ReplayCollectStates::new();
    let mut pc = StepIdx::new(0);
    for _ in 0..100 {
        let node = plan.node(pc).ok_or_else(|| format!("missing node {pc:?}"))?;
        match replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states)
            .map_err(|error| format!("{error:?}"))?
        {
            ReplayAction::Continue(next) => pc = next,
            ReplayAction::Finished => {
                let result = *run
                    .read_slot(SlotIdx::new(2))
                    .map_err(|error| format!("{error}"))?;
                let values = match result {
                    SlotValue::List(id) => store
                        .list(id)
                        .map_err(|error| format!("{error:?}"))?
                        .to_vec(),
                    _ => Vec::new(),
                };
                return Ok((result, values));
            }
            ReplayAction::Suspended { .. } => return Err("unexpected replay suspension".into()),
            _ => return Err("unexpected replay action".into()),
        }
    }
    Err("replay exceeded 100 steps".into())
}

fn step_count(events: &[EvidenceEvent], target: StepIdx, started: bool) -> usize {
    events
        .iter()
        .filter(|event| match (event, started) {
            (EvidenceEvent::StepStarted { step }, true) => *step == target,
            (EvidenceEvent::StepSucceeded { step, .. }, false) => *step == target,
            _ => false,
        })
        .count()
}

#[test]
fn together_budget_one_resumes_each_lane_once_and_matches_replay() -> Result<(), String> {
    let (signal, events, values) = run_budget_one(false)?;
    if !matches!(signal, RuntimeSignal::Finished(SlotValue::List(_))) {
        return Err(format!("unexpected signal: {signal:?}"));
    }
    let (full_signal, full_values) = drive_ir(&plan(false)?)?;
    assert!(matches!(full_signal, RuntimeSignal::Finished(_)));
    assert_eq!(values, vec![SlotValue::I64(10), SlotValue::I64(20)]);
    assert_eq!(values, full_values);
    let (replay_value, replay_values) = replay_result(false)?;
    assert!(matches!(replay_value, SlotValue::List(_)));
    assert_eq!(replay_values, values);
    for step in [StepIdx::new(1), StepIdx::new(2), StepIdx::new(3), StepIdx::new(4)] {
        assert_eq!(step_count(&events, step, true), 1, "start {step:?}");
        assert_eq!(step_count(&events, step, false), 1, "success {step:?}");
    }
    for (value, expected) in [(SlotValue::I64(10), 1), (SlotValue::I64(20), 1)] {
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, EvidenceEvent::SlotWritten { slot, value: written, .. } if *slot == SlotIdx::new(1) && *written == value))
                .count(),
            expected
        );
    }
    Ok(())
}

#[test]
fn first_success_budget_one_resumes_and_matches_replay() -> Result<(), String> {
    let (signal, events, _) = run_budget_one(true)?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(10)));
    assert_eq!(replay_result(true)?.0, SlotValue::I64(10));
    assert_eq!(step_count(&events, StepIdx::new(1), true), 1);
    assert_eq!(step_count(&events, StepIdx::new(1), false), 1);
    Ok(())
}
