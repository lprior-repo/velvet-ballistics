#![forbid(unsafe_code)]

use vb_core::{CompiledNodeKind, SlotIdx, SlotValue, StepIdx, Taint};

use super::support::{TWO, TestResult, compile, drive, finish_result, slots};
use vb_runtime::engine::{EvidenceCollector, RuntimeSignal};

fn input_sources(workflow: &vb_core::CompiledWorkflow) -> TestResult<Vec<SlotIdx>> {
    let mut sources = Vec::new();
    for raw in 0..=u16::MAX {
        let Some(node) = workflow.node(StepIdx::new(raw)) else {
            continue;
        };
        if let CompiledNodeKind::Copy { source } = node.kind {
            sources.push(source);
        }
    }
    Ok(sources)
}

/// `source_reduce_two_sequential_activations_match_live_and_replay`
#[test]
fn source_reduce_two_sequential_activations_match_live_and_replay() -> TestResult {
    let workflow = compile(TWO)?;
    let sources = input_sources(&workflow)?;
    let first = *sources.first().ok_or("first Reduce source missing")?;
    let second_source = *sources.get(1).ok_or("second Reduce source missing")?;
    let (mut frame, mut store, _) = super::support::bind(
        &workflow,
        first,
        &[SlotValue::I64(3), SlotValue::I64(7)],
        Taint::Clean,
        &[],
    )?;
    let second = store.insert_list(vec![SlotValue::I64(4)].into_boxed_slice())?;
    frame.write_slot_with_taint(second_source, SlotValue::List(second), Taint::Clean)?;
    let mut evidence = EvidenceCollector::new();
    let signal = drive(&workflow, &mut frame, &mut store, 128, &mut evidence)?;
    assert!(
        matches!(signal, RuntimeSignal::Finished(SlotValue::I64(1))),
        "signal: {signal:?}"
    );
    assert_eq!(
        frame.read_slot(finish_result(&workflow)?),
        Ok(&SlotValue::I64(1))
    );
    let live_slots = slots(&frame)?;

    let mut replay_frame = super::support::restore_frame(frame.checkpoint_v1())?;
    let mut replay_store = super::support::restore_store(store.checkpoint_v1())?;
    super::support::replay(&workflow, &mut replay_frame, &mut replay_store)?;
    assert_eq!(replay_frame.pc(), frame.pc());
    assert_eq!(slots(&replay_frame)?, live_slots);
    assert_eq!(
        replay_frame.read_slot(finish_result(&workflow)?),
        Ok(&SlotValue::I64(1))
    );
    Ok(())
}
