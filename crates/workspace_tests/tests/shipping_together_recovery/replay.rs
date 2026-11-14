#![forbid(unsafe_code)]
//! Deterministic replay parity against the live Together execution.

use super::support::*;
use vb_compile::compile_workflow;
use vb_core::replay::step::ReplayAction;
use vb_core::replay::{ReplayEngine, replay_step};
use vb_core::{
    CapabilitySet, RunFrame, RunId, SlotIdx, SlotValue, StepBudget, Taint, ValueStore,
    new_run_frame,
};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

#[test]
fn replay_engine_matches_live_together_slots_and_counters() -> TestResult {
    let plan = compile_workflow(FANOUT.as_bytes()).map_err(|error| format!("{error:?}"))?;
    let live = live(FANOUT, Box::new([]))?;
    let live_state = &live.state;
    let result_slot = live_state
        .frame
        .initialized_slots()?
        .into_iter()
        .find_map(|(slot, value, _)| matches!(value, SlotValue::List(_)).then_some(slot))
        .ok_or("live join output missing")?;
    let live_values = list_value(&live_state.store, *live_state.frame.read_slot(result_slot)?)?;

    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    let target = live_state.frame.pc();
    let replayed = engine
        .replay_frame_up_to(target, &mut store)
        .map_err(|error| format!("replay failed: {error:?}"))?;
    assert_eq!(replayed.pc(), live_state.frame.pc(), "replay pc parity");
    assert_eq!(
        replayed.executed(),
        live_state.frame.executed(),
        "replay executed-counter parity"
    );
    let replayed_values = list_value(&store, *replayed.read_slot(result_slot)?)?;
    assert_eq!(
        replayed_values, live_values,
        "replay reconstructs join order and values"
    );
    assert_eq!(
        list_taints(&store, *replayed.read_slot(result_slot)?)?,
        list_taints(&live_state.store, *live_state.frame.read_slot(result_slot)?)?
    );
    Ok(())
}

/// Seeds the declared secret input, then records the pre-execution state so the
/// live engine and the deterministic replay helpers start from identical state.
fn seeded_tainted_plan() -> TestResult<(vb_core::CompiledWorkflow, RunFrame, ValueStore, SlotIdx)> {
    let plan = tainted_plan()?;
    let hidden = hidden_slot(&plan)?;
    let mut frame = new_run_frame(RunId::new(8_102), &plan)?;
    frame.write_slot_with_taint(hidden, SlotValue::I64(23), Taint::Secret)?;
    let result_slot = finish_result(&plan)?;
    Ok((plan, frame, ValueStore::new(), result_slot))
}

#[test]
fn replay_steps_preserve_together_join_order_and_taints() -> TestResult {
    let (plan, initial_frame, initial_store, result_slot) = seeded_tainted_plan()?;
    let mut live_frame = initial_frame.clone();
    let mut live_store = initial_store.clone();
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    assert_eq!(
        drive_deterministic_full(
            &plan,
            &mut live_frame,
            &mut StepBudget::new(64),
            &mut live_store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &CapabilitySet::empty(),
            1,
        )?,
        RuntimeSignal::Finished(live_frame.read_slot(result_slot)?.clone())
    );

    let mut replayed_frame = initial_frame;
    let mut replayed_store = initial_store;
    let mut finished = false;
    for _ in 0..64 {
        let node = plan
            .node(replayed_frame.pc())
            .ok_or("replay pc outside the compiled workflow")?;
        match replay_step(node, &mut replayed_frame, &mut replayed_store, &plan)
            .map_err(|error| format!("replay step failed: {error:?}"))?
        {
            ReplayAction::Continue(_) => {}
            ReplayAction::Finished => {
                finished = true;
                break;
            }
            ReplayAction::Suspended { step, .. } => {
                return Err(format!("unexpected suspension at {step:?}").into());
            }
            _ => return Err("unexpected replay action".into()),
        }
    }
    assert!(
        finished,
        "deterministic replay must reach the terminal step"
    );
    assert_eq!(
        replayed_frame.pc(),
        live_frame.pc(),
        "replay and live end at the same step"
    );
    assert_eq!(
        replayed_frame.executed(),
        live_frame.executed(),
        "replay and live execute the same number of transitions"
    );
    assert_eq!(
        replayed_frame.parallel_in_flight(),
        0,
        "replay join releases its reservation"
    );

    let lived = *live_frame.read_slot(result_slot)?;
    let replayed = *replayed_frame.read_slot(result_slot)?;
    assert_eq!(
        list_value(&replayed_store, replayed)?,
        vec![SlotValue::I64(23), SlotValue::I64(11)],
        "replay preserves branch declaration order and branch values"
    );
    assert_eq!(
        list_value(&replayed_store, replayed)?,
        list_value(&live_store, lived)?,
        "replay join result equals the live join result"
    );
    assert_eq!(
        list_taints(&replayed_store, replayed)?,
        vec![Taint::Secret, Taint::Clean],
        "replay keeps per-item branch taints"
    );
    assert_eq!(
        replayed_frame.read_taint(result_slot)?,
        Taint::Secret,
        "replay join taint is the join of both branch taints"
    );
    Ok(())
}
