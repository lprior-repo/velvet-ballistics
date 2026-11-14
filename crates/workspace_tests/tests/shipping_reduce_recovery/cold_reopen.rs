#![forbid(unsafe_code)]

use vb_core::{
    RunFrame, SlotIdx, SlotValue, StepIdx, StepState, Taint, ValueStore, replay::step::ReplayAction,
};
use vb_runtime::engine::{EvidenceCollector, RuntimeSignal};

use super::support::{
    INPUT, SOURCE, TestResult, accumulator, bind, compile, drive, reduce_nodes, replay, replay_one,
    slots,
};

fn value(raw: i64) -> SlotValue {
    SlotValue::I64(raw)
}

fn source_values() -> Vec<SlotValue> {
    vec![value(1), value(2), value(3), value(4), value(5)]
}

fn fresh(
    workflow: &vb_core::CompiledWorkflow,
    values: &[SlotValue],
) -> TestResult<(RunFrame, ValueStore)> {
    let (frame, store, _) = bind(workflow, SlotIdx::new(INPUT), values, Taint::Clean, &[])?;
    Ok((frame, store))
}

/// `source_reduce_cold_reopen_continuation_matches_uninterrupted_at_every_boundary`
#[test]
fn source_reduce_cold_reopen_continuation_matches_uninterrupted_at_every_boundary() -> TestResult {
    let workflow = compile(SOURCE)?;
    let nodes = reduce_nodes(&workflow)?;
    let boundaries = [nodes.start, nodes.next];
    assert!(
        boundaries.len() > 1,
        "boundary loop must execute multiple stops"
    );

    let (mut reference, mut reference_store) = fresh(&workflow, &source_values())?;
    assert!(matches!(
        drive(
            &workflow,
            &mut reference,
            &mut reference_store,
            128,
            &mut EvidenceCollector::new()
        )?,
        RuntimeSignal::Finished(SlotValue::I64(15))
    ));
    let reference_states = reference.states_snapshot();

    for boundary in boundaries {
        let stop = u64::from(boundary.get())
            .checked_add(1)
            .ok_or("stop overflow")?;
        let (mut frame, mut store) = fresh(&workflow, &source_values())?;
        assert_eq!(
            drive(
                &workflow,
                &mut frame,
                &mut store,
                stop,
                &mut EvidenceCollector::new()
            )?,
            RuntimeSignal::StepBudgetExhausted,
            "boundary stop must be an explicit budget signal"
        );
        assert_eq!(frame.executed(), stop, "stop count");
        assert!(
            frame.pc().get() > nodes.start.get(),
            "stop must advance past ReduceStart"
        );

        let checkpoint = frame.checkpoint_v1();
        let store_checkpoint = store.checkpoint_v1();
        let mut reopened = super::support::restore_frame(checkpoint.clone())?;
        let mut reopened_store = super::support::restore_store(store_checkpoint.clone())?;
        let signal = drive(
            &workflow,
            &mut reopened,
            &mut reopened_store,
            128,
            &mut EvidenceCollector::new(),
        )?;
        assert!(matches!(
            signal,
            RuntimeSignal::Finished(SlotValue::I64(15))
        ));
        assert_eq!(reopened.states_snapshot(), reference_states);
        assert_eq!(slots(&reopened)?, slots(&reference)?);
        assert_eq!(reopened.pc(), reference.pc());
        assert_eq!(reopened.executed(), reference.executed());
        assert_eq!(
            reopened_store.checkpoint_v1(),
            reference_store.checkpoint_v1()
        );

        let mut replay_frame = super::support::restore_frame(checkpoint.clone())?;
        let mut replay_store = super::support::restore_store(store_checkpoint.clone())?;
        let restored_states = replay_frame.states_snapshot();
        replay(&workflow, &mut replay_frame, &mut replay_store)?;
        assert_eq!(replay_frame.pc(), reference.pc());
        assert_eq!(replay_frame.executed(), reference.executed());
        assert_eq!(slots(&replay_frame)?, slots(&reference)?);
        assert_eq!(
            replay_frame.read_slot(accumulator(&workflow)?),
            Ok(&value(15))
        );
        assert_eq!(
            replay_frame.states_snapshot(),
            restored_states,
            "replay must not mutate serialized step states"
        );

        let mut step_frame = super::support::restore_frame(checkpoint)?;
        let mut step_store = super::support::restore_store(store_checkpoint)?;
        let mut states = vb_core::replay::step::ReplayCollectStates::new();
        for _ in 0..2 {
            assert!(matches!(
                replay_one(&workflow, &mut step_frame, &mut step_store, &mut states)?,
                ReplayAction::Continue(_) | ReplayAction::Finished
            ));
        }
        assert!(matches!(
            replay_one(&workflow, &mut step_frame, &mut step_store, &mut states)?,
            ReplayAction::Continue(_) | ReplayAction::Finished
        ));
    }
    Ok(())
}

/// `source_reduce_cold_reopen_parity_for_empty_single_and_multiple`
#[test]
fn source_reduce_cold_reopen_parity_for_empty_single_and_multiple() -> TestResult {
    let workflow = compile(SOURCE)?;
    for values in [Vec::new(), vec![value(3)], vec![value(3), value(7)]] {
        let expected = 10 + i64::try_from(values.len())?;
        let (mut frame, mut store) = fresh(&workflow, &values)?;
        assert_eq!(
            drive(
                &workflow,
                &mut frame,
                &mut store,
                1,
                &mut EvidenceCollector::new()
            )?,
            RuntimeSignal::StepBudgetExhausted
        );
        let mut reopened = super::support::restore_frame(frame.checkpoint_v1())?;
        let mut reopened_store = super::support::restore_store(store.checkpoint_v1())?;
        assert!(matches!(
            drive(
                &workflow,
                &mut reopened,
                &mut reopened_store,
                128,
                &mut EvidenceCollector::new()
            )?,
            RuntimeSignal::Finished(_)
        ));
        assert_eq!(
            reopened.read_slot(accumulator(&workflow)?),
            Ok(&value(expected))
        );
        assert_eq!(
            reopened.step_state(StepIdx::new(0)),
            Ok(StepState::Succeeded)
        );
    }
    Ok(())
}
