use super::*;
use vb_core::{CapabilitySet, EngineError, RunFrame, StepBudget, StepState, Taint, ValueStore};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal,
    drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

fn check(family: u8, executed: u64) -> TestResult {
    let (parts, start) = match family {
        0 => (fixture(false, true)?, StepIdx::new(1)),
        1 => (siblings::reduction(false)?, StepIdx::new(2)),
        2 => (siblings::together(false)?, StepIdx::new(6)),
        _ => return Err("family".into()),
    };
    let plan = CompiledWorkflow::try_from_parts(parts.clone())?;
    vb_validate::shared::validate(&parts)?;
    let node = plan.node(start).ok_or("start")?;
    let mut run = RunFrame::new(RunId::new(1), start, plan.node_count(), plan.slot_count())?;
    let mut store = ValueStore::with_max_slots(plan.resource_contract().max_slots);
    let list = store.insert_list(Box::from([SlotValue::I64(7)]))?;
    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::List(list), Taint::Secret)?;
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(42), Taint::TimeDependent)?;
    let mut checkpoint = run.checkpoint_v1();
    checkpoint.executed = executed;
    let mut run = RunFrame::from_checkpoint_v1(checkpoint)
        .map_err(|error| format!("checked frame: {error:?}"))?;
    let before = run.initialized_slots()?;
    let before_store = store.checkpoint_v1();
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    let result = drive_deterministic_full(
        &plan,
        &mut run,
        &mut StepBudget::new(1),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect,
        &CapabilitySet::empty(),
        1,
    );
    if executed == u64::MAX {
        assert!(matches!(
            result,
            Err(RuntimeEngineError::Core(EngineError::StepCounterOverflow))
        ));
        assert_eq!(run.initialized_slots()?, before);
        assert_eq!(store.checkpoint_v1(), before_store);
        assert_eq!(run.parallel_in_flight(), 0);
        assert_eq!(run.executed(), u64::MAX);
        assert_eq!(run.pc(), node.routing.error.handler().ok_or("handler")?);
        assert_eq!(run.step_state(start)?, StepState::Failed);
        assert_eq!(
            evidence.drain(),
            vec![EvidenceEvent::StepStarted { step: start }]
        );
    } else {
        assert_eq!(result?, RuntimeSignal::StepBudgetExhausted);
        assert_eq!(run.executed(), u64::MAX);
        assert_eq!(run.step_state(start)?, StepState::Succeeded);
        assert_eq!(
            run.read_slot(SlotIdx::new(2))?,
            &SlotValue::List(vb_core::ListId::new(1))
        );
        assert_eq!(
            run.read_taint(SlotIdx::new(2))?,
            if family == 2 {
                Taint::Clean
            } else {
                Taint::Secret
            }
        );
        let expected = match node.kind {
            K::ForEachStart { body, .. } | K::ReduceStart { body, .. } => body,
            K::TogetherStart { ref branches, .. } => *branches.first().ok_or("branch")?,
            _ => return Err("start kind".into()),
        };
        assert_eq!(run.pc(), expected);
        assert_eq!(run.parallel_in_flight(), if family == 2 { 1 } else { 0 });
    }
    Ok(())
}

#[test]
fn counter_max_preflight_preserves_start_prestate_and_router_pc_commit() -> TestResult {
    for family in 0..3 {
        check(family, u64::MAX)?;
    }
    Ok(())
}

#[test]
fn counter_max_minus_one_commits_only_the_successful_start_edge() -> TestResult {
    for family in 0..3 {
        check(family, u64::MAX - 1)?;
    }
    Ok(())
}
