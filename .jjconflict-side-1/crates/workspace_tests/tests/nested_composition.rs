//! End-to-end nested primitive composition: compile nested YAML, then drive it.
//!
//! Bead: vb-hkvsh. Compiles `for_each`/`repeat`/`choose` nests with
//! `compile_workflow`, binds declared list inputs, drives with
//! `drive_deterministic_full`, and asserts terminal values plus per-node
//! execution counts. Determinism is checked by driving every workflow twice.

#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledNodeKind, RunId, SlotIdx, SlotValue, StepBudget, Taint, ValueStore,
    new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

type TestResult = Result<(), String>;
type Driven = (RuntimeSignal, Vec<EvidenceEvent>, Vec<Vec<SlotValue>>);

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: nested_composition\nwhen: {manual: {}}\n";
const INPUTS: &str = "inputs:\n  items: {is: list, of: any, max: 4}\n";

fn drive_source(source: &str, items: &[i64]) -> Result<Driven, String> {
    let workflow =
        compile_workflow(source.as_bytes()).map_err(|error| format!("compile: {error:?}"))?;
    let mut frame =
        new_run_frame(RunId::new(1), &workflow).map_err(|error| format!("frame: {error:?}"))?;
    let mut store = ValueStore::new();
    let values: Vec<SlotValue> = items.iter().copied().map(SlotValue::I64).collect();
    let list = store
        .insert_list(values)
        .map_err(|error| format!("input list: {error:?}"))?;
    let input_slot = workflow
        .input_slots()
        .first()
        .ok_or_else(|| "missing declared input slot".to_owned())?
        .slot;
    frame
        .write_slot_with_taint(input_slot, SlotValue::List(list), Taint::Secret)
        .map_err(|error| format!("input binding: {error:?}"))?;
    let mut evidence = EvidenceCollector::new();
    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(256),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|error| format!("drive: {error:?}"))?;
    let events = evidence.drain();
    let mut aggregates = Vec::new();
    for event in &events {
        let EvidenceEvent::SlotWritten {
            slot,
            value: SlotValue::List(list),
            ..
        } = event
        else {
            continue;
        };
        if workflow.to_parts().nodes.iter().any(|node| {
            matches!(node.kind, CompiledNodeKind::ForEachJoin { .. })
                && node.output.slot() == Some(*slot)
        }) {
            aggregates.push(
                store
                    .list(*list)
                    .map_err(|error| format!("aggregate: {error:?}"))?
                    .to_vec(),
            );
        }
    }
    Ok((signal, events, aggregates))
}

fn drive_twice(source: &str, items: &[i64]) -> Result<Driven, String> {
    let first = drive_source(source, items)?;
    let second = drive_source(source, items)?;
    if first != second {
        return Err("drive signal drifted between runs".to_owned());
    }
    assert_eq!(first.2, second.2);
    Ok(first)
}

fn start_count(events: &[EvidenceEvent], step: u16) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                EvidenceEvent::StepStarted { step: started } if started.get() == step
            )
        })
        .count()
}

fn slot_writes(events: &[EvidenceEvent], slot: SlotIdx) -> Vec<SlotValue> {
    events
        .iter()
        .filter_map(|event| match event {
            EvidenceEvent::SlotWritten {
                slot: written,
                value,
                ..
            } if *written == slot => Some(*value),
            _ => None,
        })
        .collect()
}

fn for_each_repeat_source() -> String {
    format!(
        "{HEADER}{INPUTS}steps:\n  - id: seed\n    set: {{output: base, value: '100'}}\n  - id: outer\n    for_each:\n      input: '$input.items'\n      variable: entry\n      steps:\n        - id: inner\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: tick\n                set: {{output: tick_value, value: '$entry'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

#[test]
fn nested_repeat_runs_per_outer_item() -> TestResult {
    let source = for_each_repeat_source();
    let (signal, events, aggregates) = drive_twice(&source, &[1, 2])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(100)));
    // Outer body entered twice, inner tick set runs 2 attempts per item.
    assert_eq!(start_count(&events, 3), 1);
    assert_eq!(start_count(&events, 4), 2);
    assert_eq!(start_count(&events, 5), 4);
    assert_eq!(start_count(&events, 6), 4);
    assert_eq!(start_count(&events, 8), 2);
    assert_eq!(aggregates, [vec![SlotValue::I64(1), SlotValue::I64(2)]]);
    Ok(())
}

#[test]
fn nested_repeat_skipped_item_counts_stay_zero() -> TestResult {
    let (signal, events, aggregates) = drive_twice(&for_each_repeat_source(), &[])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(100)));
    assert_eq!(start_count(&events, 3), 1);
    assert_eq!(start_count(&events, 5), 0);
    assert_eq!(aggregates, [Vec::<SlotValue>::new()]);
    Ok(())
}

fn repeat_for_each_source() -> String {
    format!(
        "{HEADER}{INPUTS}steps:\n  - id: seed\n    set: {{output: base, value: '100'}}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: sub\n          for_each:\n            input: '$input.items'\n            variable: entry\n            steps:\n              - id: leaf\n                set: {{output: picked, value: '$entry'}}\n        - id: grab\n          set: {{output: grabbed, value: '$step.sub'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

#[test]
fn nested_for_each_value_threads_through_repeat() -> TestResult {
    let source = repeat_for_each_source();
    let workflow =
        compile_workflow(source.as_bytes()).map_err(|error| format!("compile: {error:?}"))?;
    let (signal, events, aggregates) = drive_twice(&source, &[5, 6])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(100)));
    // Each Repeat activation publishes the inner aggregate and copies it in grab.
    assert_eq!(start_count(&events, 5), 4);
    assert_eq!(start_count(&events, 9), 2);
    assert_eq!(
        aggregates,
        [
            vec![SlotValue::I64(5), SlotValue::I64(6)],
            vec![SlotValue::I64(5), SlotValue::I64(6)]
        ]
    );
    let leaf_slot = workflow
        .node(vb_core::StepIdx::new(5))
        .ok_or_else(|| "missing leaf node".to_owned())?
        .output
        .slot()
        .ok_or_else(|| "missing leaf output".to_owned())?;
    assert_eq!(
        slot_writes(&events, leaf_slot),
        [
            SlotValue::I64(5),
            SlotValue::I64(6),
            SlotValue::I64(5),
            SlotValue::I64(6),
        ]
    );
    Ok(())
}

fn choose_nested_source() -> String {
    format!(
        "{HEADER}{INPUTS}steps:\n  - id: seed\n    set: {{output: base, value: '100'}}\n  - id: flag\n    set: {{output: go, value: 'true'}}\n  - id: outer\n    for_each:\n      input: '$input.items'\n      variable: entry\n      steps:\n        - id: route\n          choose:\n            branches:\n              - when: '$flag'\n                steps:\n                  - id: inner\n                    repeat:\n                      max_attempts: 2\n                      steps:\n                        - id: tick\n                          set: {{output: tick_value, value: '$entry'}}\n            otherwise: after_route\n        - id: after_route\n          set: {{output: after_route, value: '$entry'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

#[test]
fn nested_repeat_inside_choose_branch_runs() -> TestResult {
    let (signal, events, aggregates) = drive_twice(&choose_nested_source(), &[7])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(100)));
    assert_eq!(start_count(&events, 5), 1);
    assert_eq!(start_count(&events, 7), 2);
    assert_eq!(start_count(&events, 13), 1);
    assert_eq!(aggregates, [vec![SlotValue::I64(7)]]);
    Ok(())
}
