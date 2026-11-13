#![forbid(unsafe_code)]

use vb_core::{
    CompiledNodeKind, SlotValue, StepIdx, ValueStore, new_run_frame,
    replay::ReplayError,
    replay::step::{ReplayCollectStates, replay_step_with_collect},
};
use vb_runtime::primitives::reduce::reduce_next;

use super::support::{SOURCE, TestResult, accumulator, compile, iterator_slot, reduce_nodes};

fn value(raw: i64) -> SlotValue {
    SlotValue::I64(raw)
}

fn reduce_next_node(workflow: &vb_core::CompiledWorkflow) -> TestResult<&vb_core::CompiledNode> {
    (0..=u16::MAX)
        .find_map(|raw| {
            let node = workflow.node(StepIdx::new(raw))?;
            matches!(node.kind, CompiledNodeKind::ReduceNext { .. }).then_some(node)
        })
        .ok_or_else(|| "ReduceNext absent".into())
}

/// `source_reduce_replay_rejects_corrupt_iterator_handle`
#[test]
fn source_reduce_replay_rejects_corrupt_iterator_handle() -> TestResult {
    let workflow = compile(SOURCE)?;
    let iterator = iterator_slot(&workflow)?;
    let node = reduce_next_node(&workflow)?;
    let cases: [(&str, Vec<SlotValue>); 3] = [
        ("empty handle list", Vec::new()),
        (
            "three-element handle list",
            vec![value(0), value(0), value(0)],
        ),
        (
            "dangling source ListId",
            vec![value(i64::from(u32::MAX)), value(0)],
        ),
    ];
    for (reason, state) in cases {
        let mut frame = new_run_frame(vb_core::RunId::new(1), &workflow)?;
        let mut store = ValueStore::new();
        let list = store.insert_list(state.into_boxed_slice())?;
        frame.write_slot(iterator, SlotValue::List(list))?;
        let frame_before = frame.checkpoint_v1();
        let store_before = store.checkpoint_v1();
        let mut states = ReplayCollectStates::new();
        let result = replay_step_with_collect(node, &mut frame, &mut store, &workflow, &mut states);
        match result {
            Err(ReplayError::Internal { .. }) => {}
            _ => return Err(format!("{reason}: expected typed replay refusal").into()),
        }
        assert_eq!(frame.checkpoint_v1(), frame_before, "{reason}: frame moved");
        assert_eq!(store.checkpoint_v1(), store_before, "{reason}: store moved");
    }
    Ok(())
}

/// `source_reduce_live_rejects_corrupt_iterator_handle_typed`
#[test]
fn source_reduce_live_rejects_corrupt_iterator_handle_typed() -> TestResult {
    let workflow = compile(SOURCE)?;
    let iterator = iterator_slot(&workflow)?;
    let nodes = reduce_nodes(&workflow)?;
    let acc = accumulator(&workflow)?;
    let node = reduce_next_node(&workflow)?;
    let item_output = node.output.slot();
    let cases: [(&str, Vec<SlotValue>); 3] = [
        ("empty handle list", Vec::new()),
        (
            "three-element handle list",
            vec![value(0), value(0), value(0)],
        ),
        (
            "dangling source ListId",
            vec![value(i64::from(u32::MAX)), value(0)],
        ),
    ];
    for (reason, state) in cases {
        let mut frame = new_run_frame(vb_core::RunId::new(1), &workflow)?;
        let mut store = ValueStore::new();
        let list = store.insert_list(state.into_boxed_slice())?;
        frame.write_slot(iterator, SlotValue::List(list))?;
        let frame_before = frame.checkpoint_v1();
        let store_before = store.checkpoint_v1();
        let result = reduce_next(
            &mut frame,
            &mut store,
            iterator,
            acc,
            nodes.body,
            nodes.done,
            item_output,
        );
        match result {
            Err(vb_core::errors::EngineError::ListOutOfBounds { .. })
                if reason == "dangling source ListId" => {}
            Err(vb_core::errors::EngineError::InternalInvariantViolation { .. })
                if reason != "dangling source ListId" => {}
            _ => return Err(format!("{reason}: expected typed live refusal").into()),
        }
        assert_eq!(frame.checkpoint_v1(), frame_before, "{reason}: frame moved");
        assert_eq!(store.checkpoint_v1(), store_before, "{reason}: store moved");
    }
    Ok(())
}
