#![forbid(unsafe_code)]

use vb_core::{SlotIdx, SlotValue, StepState, Taint, new_run_frame};
use vb_runtime::engine::{EvidenceCollector, EvidenceEvent, RuntimeSignal};

use super::support::{
    INPUT, SOURCE, TestResult, accumulator, assert_source, bind, compile, drive, finish_result,
    iterator_slot, reduce_input_slot, reduce_next_item_slot, reduce_nodes, replay, replay_one,
    slots,
};

fn value(raw: i64) -> SlotValue {
    SlotValue::I64(raw)
}

/// `source_reduce_live_and_replay_parity_for_empty_single_and_multiple`
#[test]
fn source_reduce_live_and_replay_parity_for_empty_single_and_multiple() -> TestResult {
    for values in [Vec::new(), vec![value(3)], vec![value(3), value(7)]] {
        let workflow = compile(SOURCE)?;
        let (mut frame, mut store, list) =
            bind(&workflow, SlotIdx::new(INPUT), &values, Taint::Clean, &[])?;
        let before = slots(&frame)?;
        let mut evidence = EvidenceCollector::new();
        let expected = 10 + i64::try_from(values.len())?;
        let signal = drive(&workflow, &mut frame, &mut store, 128, &mut evidence)?;
        assert!(
            matches!(signal, RuntimeSignal::Finished(SlotValue::I64(value)) if value == expected),
            "signal: {signal:?}"
        );
        assert_source(&frame, &store, list, &values, Taint::Clean);
        let nodes = reduce_nodes(&workflow)?;
        assert_eq!(frame.step_state(nodes.start), Ok(StepState::Succeeded));
        let loop_state = if values.is_empty() {
            StepState::Pending
        } else {
            StepState::Succeeded
        };
        assert_eq!(frame.step_state(nodes.body), Ok(loop_state));
        assert_eq!(frame.step_state(nodes.next), Ok(loop_state));
        assert_eq!(frame.step_state(nodes.finish), Ok(StepState::Succeeded));
        assert_eq!(
            frame.read_slot(accumulator(&workflow)?),
            Ok(&SlotValue::I64(expected))
        );
        assert_eq!(
            frame.read_slot(finish_result(&workflow)?),
            Ok(&SlotValue::I64(expected))
        );

        let mut replay_frame = super::support::restore_frame(frame.checkpoint_v1())?;
        let mut replay_store = super::support::restore_store(store.checkpoint_v1())?;
        replay(&workflow, &mut replay_frame, &mut replay_store)?;
        assert_eq!(replay_frame.pc(), frame.pc());
        assert_eq!(slots(&replay_frame)?, slots(&frame)?);
        assert_eq!(replay_store.checkpoint_v1(), store.checkpoint_v1());
        assert_eq!(
            replay_frame.read_slot(accumulator(&workflow)?),
            Ok(&SlotValue::I64(expected))
        );
        assert_eq!(
            replay_frame.read_slot(finish_result(&workflow)?),
            Ok(&SlotValue::I64(expected))
        );
        assert_eq!(
            replay_frame.step_state(nodes.start),
            Ok(StepState::Succeeded),
            "replay preserves the serialized start state"
        );
        for step in [nodes.body, nodes.next] {
            assert_eq!(
                replay_frame.step_state(step),
                Ok(loop_state),
                "replay preserves the serialized state and never marks completed steps"
            );
        }
        assert_eq!(
            replay_frame.step_state(nodes.finish),
            Ok(StepState::Succeeded),
            "ReduceFinish completed in the serialized frame"
        );
        assert!(before.len() < slots(&frame)?.len());
    }
    Ok(())
}

/// `source_reduce_live_and_replay_evidence_names_accumulator_and_iterator`
#[test]
fn source_reduce_live_and_replay_evidence_names_accumulator_and_iterator() -> TestResult {
    let workflow = compile(SOURCE)?;
    let acc = accumulator(&workflow)?;
    let iterator = iterator_slot(&workflow)?;
    let mut frame = new_run_frame(vb_core::RunId::new(1), &workflow)?;
    let (_, mut store, list) = bind(
        &workflow,
        SlotIdx::new(INPUT),
        &[value(3), value(7)],
        Taint::Clean,
        &[],
    )?;
    frame.write_slot(SlotIdx::new(INPUT), SlotValue::List(list))?;
    let mut evidence = EvidenceCollector::new();
    let signal = drive(&workflow, &mut frame, &mut store, 128, &mut evidence)?;
    assert!(matches!(signal, RuntimeSignal::Finished(_)));
    let events = evidence.drain();
    assert!(events.iter().any(|event| matches!(
        event,
        EvidenceEvent::SlotWritten { slot, value: SlotValue::List(_), taint, .. }
            if *slot == iterator && *taint == Taint::Clean
    )));
    assert!(events.iter().any(|event| {
        let observed = match event {
            EvidenceEvent::SlotWritten { slot, value, .. } => (*slot, *value),
            _ => return false,
        };
        observed.0 == acc && observed.1 == value(12)
    }));
    Ok(())
}

/// `source_reduce_preserves_source_taints_across_all_variants`
#[test]
fn source_reduce_preserves_source_taints_across_all_variants() -> TestResult {
    for taint in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ] {
        let workflow = compile(SOURCE)?;
        let (mut frame, mut store, list) = bind(
            &workflow,
            SlotIdx::new(INPUT),
            &[value(3), value(7)],
            taint,
            &[Taint::Clean, Taint::Clean],
        )?;
        let first = reduce_input_slot(&workflow)?;
        let second = reduce_next_item_slot(&workflow)?;
        let mut states = vb_core::replay::step::ReplayCollectStates::new();
        assert!(matches!(
            replay_one(&workflow, &mut frame, &mut store, &mut states)?,
            vb_core::replay::step::ReplayAction::Continue(_)
        ));
        assert!(matches!(
            replay_one(&workflow, &mut frame, &mut store, &mut states)?,
            vb_core::replay::step::ReplayAction::Continue(_)
        ));
        assert_eq!(frame.read_slot(first), Ok(&value(3)));
        assert_eq!(frame.read_taint(first), Ok(taint));
        assert!(matches!(
            replay_one(&workflow, &mut frame, &mut store, &mut states)?,
            vb_core::replay::step::ReplayAction::Continue(_)
        ));
        assert!(matches!(
            replay_one(&workflow, &mut frame, &mut store, &mut states)?,
            vb_core::replay::step::ReplayAction::Continue(_)
        ));
        assert_eq!(frame.read_slot(second), Ok(&value(7)));
        assert_eq!(frame.read_taint(second), Ok(taint));
        for _ in 0..16 {
            match replay_one(&workflow, &mut frame, &mut store, &mut states)? {
                vb_core::replay::step::ReplayAction::Continue(_) => {}
                vb_core::replay::step::ReplayAction::Finished => break,
                _ => return Err("unexpected replay suspension".into()),
            }
        }
        assert_eq!(frame.read_slot(accumulator(&workflow)?), Ok(&value(12)));
        assert_eq!(
            frame.read_taint(accumulator(&workflow)?),
            Ok(Taint::Clean),
            "accumulator joins only its source values"
        );
        assert_source(&frame, &store, list, &[value(3), value(7)], taint);
    }
    Ok(())
}
