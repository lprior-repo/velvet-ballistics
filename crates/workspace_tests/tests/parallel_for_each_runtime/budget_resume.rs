use super::*;
use vb_runtime::engine::types::EvidenceEvent;

#[test]
fn budgets_one_two_and_four_resume_each_lane_once_and_match_replay() -> Result<(), String> {
    for budget_size in [1_u64, 2, 4] {
        assert_budget_resume(budget_size)?;
    }
    Ok(())
}

fn assert_budget_resume(budget_size: u64) -> Result<(), String> {
    let plan = compile("2")?;
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), plan.node_count(), plan.slot_count())
        .map_err(|error| format!("{error}"))?;
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let granted = vb_core::capability::CapabilitySet::empty();
    let input = plan.input_slots().first().ok_or("missing input slot")?.slot;
    let list = store
        .insert_list(Box::from([
            SlotValue::I64(10),
            SlotValue::I64(20),
            SlotValue::I64(30),
        ]))
        .map_err(|error| format!("{error}"))?;
    run.write_slot(input, SlotValue::List(list))
        .map_err(|error| format!("{error}"))?;

    let (start, body, lane_end, join) = for_each_steps(&plan)?;
    let lane_output = plan
        .node(body)
        .and_then(|node| node.output.slot())
        .ok_or("missing lane output")?;
    let runtime_result = {
        let mut finished = None;
        for _ in 0..100 {
            let mut budget = StepBudget::new(budget_size);
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
            .map_err(|error| format!("{error}"))?;
            if let RuntimeSignal::Finished(SlotValue::List(id)) = signal {
                finished = Some(
                    store
                        .list(id)
                        .map_err(|error| format!("{error}"))?
                        .to_vec(),
                );
                break;
            }
            if signal != RuntimeSignal::StepBudgetExhausted {
                return Err(format!("unexpected budget-{budget_size} signal: {signal:?}"));
            }
        }
        finished.ok_or("small-budget drives did not finish")?
    };
    assert_eq!(runtime_result, vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)]);
    assert_eq!(replay(&plan, &[10, 20, 30])?, vec![10, 20, 30]);

    let events = evidence.drain();
    assert_eq!(count_started(&events, start), 1);
    assert_eq!(count_succeeded(&events, start), 1);
    assert_eq!(count_started(&events, body), 3);
    assert_eq!(count_succeeded(&events, body), 3);
    assert_eq!(count_written(&events, lane_output), 3);
    assert_eq!(count_started(&events, lane_end), 3);
    assert_eq!(count_succeeded(&events, lane_end), 3);
    assert_eq!(count_started(&events, join), 1);
    assert_eq!(count_succeeded(&events, join), 1);
    Ok(())
}

#[test]
fn concurrent_for_each_limit_error_is_typed_in_runtime_and_replay() -> Result<(), String> {
    let plan = compile("2")?;
    let runtime = drive_signal(&plan, &[1, 2, 3, 4, 5], 10_000);
    assert!(
        matches!(&runtime, Err(error) if error.contains("iteration limit exceeded") && error.contains("for_each_limit")),
        "unexpected runtime limit result: {runtime:?}"
    );
    assert_eq!(
        replay(&plan, &[1, 2, 3, 4, 5]),
        Err("IterationLimitExceeded { resource: \"for_each_limit\" }".into())
    );
    Ok(())
}

fn for_each_steps(plan: &CompiledWorkflow) -> Result<(StepIdx, StepIdx, StepIdx, StepIdx), String> {
    let mut found = None;
    for raw in 0..plan.node_count() {
        let Some(node) = plan.node(StepIdx::new(raw)) else {
            return Err(format!("missing plan node {raw}"));
        };
        if let vb_core::CompiledNodeKind::ForEachConcurrentStart { body, join, .. } = node.kind {
            let end = (0..plan.node_count())
                .find_map(|index| {
                    plan.node(StepIdx::new(index)).and_then(|candidate| {
                        matches!(
                            candidate.kind,
                            vb_core::CompiledNodeKind::ConcurrentLaneEnd { owner, .. }
                                if owner == node.id
                        )
                        .then_some(candidate.id)
                    })
                })
                .ok_or("missing ForEach lane end")?;
            found = Some((node.id, body, end, join));
            break;
        }
    }
    found.ok_or_else(|| "missing concurrent ForEach start".into())
}

fn count_started(events: &[EvidenceEvent], step: StepIdx) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, EvidenceEvent::StepStarted { step: found } if *found == step))
        .count()
}

fn count_succeeded(events: &[EvidenceEvent], step: StepIdx) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, EvidenceEvent::StepSucceeded { step: found, .. } if *found == step))
        .count()
}

fn count_written(events: &[EvidenceEvent], slot: SlotIdx) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, EvidenceEvent::SlotWritten { slot: found, .. } if *found == slot))
        .count()
}
