//! Nested repeat activation: repeated activation, check exhaustion, and
//! per-attempt rearm via source-driven drive.
//!
//! Bead: vb-eirmk.4.5. Proves packed counter ownership across nested repeats
//! and outer loops, plus check/done routes under repeated activation.

#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledNodeKind, CompiledWorkflow, RunId, SlotValue, StepBudget, StepIdx,
    Taint, ValueStore, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

type TestResult = Result<(), String>;

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: nested_repeat_activation\nwhen: {manual: {}}\n";
const INPUTS: &str = "inputs:\n  items: {is: list, of: any, max: 4}\n";

fn drive_source(
    source: &str,
    items: &[i64],
) -> Result<(RuntimeSignal, Vec<EvidenceEvent>), String> {
    let workflow = compile_workflow(source.as_bytes()).map_err(|e| format!("compile: {e:?}"))?;
    let mut frame = new_run_frame(RunId::new(1), &workflow).map_err(|e| format!("frame: {e:?}"))?;
    let mut store = ValueStore::new();
    let values: Vec<SlotValue> = items.iter().copied().map(SlotValue::I64).collect();
    if let Some(slot) = workflow.input_slots().first().map(|i| i.slot) {
        let list = store
            .insert_list(values)
            .map_err(|e| format!("list: {e:?}"))?;
        frame
            .write_slot_with_taint(slot, SlotValue::List(list), Taint::Secret)
            .map_err(|e| format!("bind: {e:?}"))?;
    }
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
    .map_err(|e| format!("drive: {e:?}"))?;
    Ok((signal, evidence.drain()))
}

fn drive_twice(source: &str, items: &[i64]) -> Result<(RuntimeSignal, Vec<EvidenceEvent>), String> {
    let first = drive_source(source, items)?;
    let second = drive_source(source, items)?;
    if first != second {
        return Err("drift".to_owned());
    }
    Ok(first)
}

fn start_count(events: &[EvidenceEvent], step: StepIdx) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepStarted { step: s } if *s == step))
        .count()
}

fn repeat_regions(
    workflow: &CompiledWorkflow,
) -> Result<Vec<vb_core::workflow::CompiledLoopRegion>, String> {
    let parts = workflow.to_parts();
    parts
        .nodes
        .iter()
        .filter_map(|node| match node.kind {
            CompiledNodeKind::RepeatStart { body, done, .. } => {
                Some((node.id, body, done, node.output.slot()))
            }
            _ => None,
        })
        .map(|(start, body, done, attempt)| {
            let attempt = attempt.ok_or("repeat attempt slot")?;
            let check = parts
                .nodes
                .iter()
                .find(|node| {
                    matches!(node.kind, CompiledNodeKind::RepeatCheck { attempt_slot, done: target }
                if attempt_slot == attempt && target == done)
                        && node.routing.next.step() == Some(body)
                })
                .ok_or("matching RepeatCheck")?;
            let region = workflow
                .loop_region(check.id)
                .copied()
                .ok_or("repeat region")?;
            assert_eq!(
                (region.start(), region.body(), region.done()),
                (start, body, done)
            );
            Ok(region)
        })
        .collect()
}

fn flat_repeat_source(attempts: u16) -> String {
    format!(
        "{HEADER}steps:\n  - id: s\n    set: {{output: x, value: '10'}}\n  - id: r\n    repeat:\n      max_attempts: {attempts}\n      steps:\n        - id: b\n          set: {{output: out, value: '$x'}}\n  - id: done\n    finish: {{result: '$x'}}\n"
    )
}

// Check success: repeat with max 2 runs body exactly twice then finishes.
#[test]
fn repeat_check_success_runs_body_exactly_max_attempts() -> TestResult {
    let source = flat_repeat_source(2);
    let workflow = compile_workflow(source.as_bytes()).map_err(|e| format!("compile: {e:?}"))?;
    let regions = repeat_regions(&workflow)?;
    let region = regions.first().ok_or("repeat region")?;
    let (signal, events) = drive_twice(&source, &[])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(10)));
    assert_eq!(start_count(&events, region.body()), 2);
    assert_eq!(start_count(&events, region.control()), 2);
    assert_eq!(start_count(&events, region.done()), 1);
    Ok(())
}

// Check exhaustion: max 1 runs once, check immediately exhausts to done.
#[test]
fn repeat_check_exhaustion_routes_to_done() -> TestResult {
    let source = flat_repeat_source(1);
    let workflow = compile_workflow(source.as_bytes()).map_err(|e| format!("compile: {e:?}"))?;
    let regions = repeat_regions(&workflow)?;
    let region = regions.first().ok_or("repeat region")?;
    let (signal, events) = drive_twice(&source, &[])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(10)));
    assert_eq!(start_count(&events, region.body()), 1);
    assert_eq!(start_count(&events, region.control()), 1);
    assert_eq!(start_count(&events, region.done()), 1);
    Ok(())
}

// Repeated activation: nested repeat under for_each rearms per outer item.
// Outer 3 items, inner max 2 => tick runs 3*2=6 times.
#[test]
fn nested_repeat_repeated_activation_resets_attempt_counter() -> TestResult {
    let source = format!(
        "{HEADER}{INPUTS}steps:\n  - id: s\n    set: {{output: b, value: '99'}}\n  - id: o\n    for_each:\n      input: '$input.items'\n      variable: e\n      steps:\n        - id: i\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: t\n                set: {{output: v, value: '$e'}}\n  - id: done\n    finish: {{result: '$b'}}\n"
    );
    let workflow = compile_workflow(source.as_bytes()).map_err(|e| format!("compile: {e:?}"))?;
    let regions = repeat_regions(&workflow)?;
    let inner = regions.first().ok_or("inner repeat")?;
    let outer = (0..workflow.node_count())
        .map(StepIdx::new)
        .find_map(|id| {
            workflow.node(id).and_then(|node| {
                matches!(node.kind, CompiledNodeKind::ForEachNext { .. })
                    .then(|| workflow.loop_region(id))
                    .flatten()
                    .copied()
            })
        })
        .ok_or("outer for_each")?;
    let (signal, events) = drive_twice(&source, &[7, 8, 9])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(99)));
    assert_eq!(start_count(&events, outer.start()), 1);
    assert_eq!(start_count(&events, inner.start()), 3);
    assert_eq!(start_count(&events, inner.body()), 6);
    assert_eq!(start_count(&events, inner.control()), 6);
    assert_eq!(start_count(&events, outer.control()), 3);
    Ok(())
}

#[test]
fn canonical_repeat_attempt_number_is_one_based_and_shadowed() -> TestResult {
    let source = format!(
        "{HEADER}steps:\n  - id: outer\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: inner\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: inner_value\n                set: {{output: observed_inner, value: '$attempt.number'}}\n        - id: combined\n          set: {{output: observed, value: '$step.inner * 100 + $attempt.number'}}\n  - id: done\n    finish:\n      result: '$step.outer'\n"
    );
    let workflow =
        compile_workflow(source.as_bytes()).map_err(|error| format!("compile: {error:?}"))?;
    let repeat_slots = workflow
        .to_parts()
        .nodes
        .iter()
        .filter_map(|node| match node.kind {
            CompiledNodeKind::RepeatStart { body, .. } => workflow.node(body)?.output.slot(),
            _ => None,
        })
        .collect::<Vec<_>>();
    let outer_slot = repeat_slots.first().copied().ok_or("outer attempt slot")?;
    let inner_slot = repeat_slots.get(1).copied().ok_or("inner attempt slot")?;
    let (signal, events) = drive_source(&source, &[])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(203)));
    let writes = |slot| {
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
            .collect::<Vec<_>>()
    };
    assert_eq!(
        writes(outer_slot),
        [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]
    );
    assert_eq!(
        writes(inner_slot),
        [
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(1),
            SlotValue::I64(2),
        ]
    );
    Ok(())
}

#[test]
fn canonical_repeat_attempt_number_inherits_through_choose() -> TestResult {
    let source = format!(
        "{HEADER}steps:\n  - id: outer\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: route\n          choose:\n            branches:\n              - when: '$attempt.number == 3'\n                steps:\n                  - id: nested_attempt\n                    set: {{output: nested_value, value: '$attempt.number'}}\n            otherwise: after_route\n        - id: after_route\n          set: {{output: outer_value, value: '$attempt.number'}}\n  - id: done\n    finish:\n      result: '$step.outer'\n"
    );
    let workflow =
        compile_workflow(source.as_bytes()).map_err(|error| format!("compile: {error:?}"))?;
    let branch_step = (0..workflow.node_count())
        .map(StepIdx::new)
        .find(|step| workflow.step_name(*step) == Some("nested_attempt"))
        .ok_or("nested choose output step")?;
    let branch_slot = workflow
        .node(branch_step)
        .and_then(|node| node.output.slot())
        .ok_or("nested choose output slot")?;
    let (signal, events) = drive_source(&source, &[])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(3)));
    assert_eq!(
        events
            .iter()
            .filter_map(|event| match event {
                EvidenceEvent::SlotWritten { slot, value, .. } if *slot == branch_slot => {
                    Some(*value)
                }
                _ => None,
            })
            .collect::<Vec<_>>(),
        [SlotValue::I64(3)]
    );
    Ok(())
}

// Triple nesting: repeat outer drives each inner activation to exhaustion.
// Verifies per-attempt rearm for multiple levels.
#[test]
fn triple_nested_repeat_per_attempt_rearm() -> TestResult {
    let source = format!(
        "{HEADER}steps:\n  - id: s\n    set: {{output: b, value: '5'}}\n  - id: o\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: m\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: i\n                repeat:\n                  max_attempts: 1\n                  steps:\n                    - id: leaf\n                      set: {{output: v, value: '1'}}\n  - id: done\n    finish: {{result: '$b'}}\n"
    );
    let workflow = compile_workflow(source.as_bytes()).map_err(|e| format!("compile: {e:?}"))?;
    let regions = repeat_regions(&workflow)?;
    let outer = regions.first().ok_or("outer repeat")?;
    let middle_start = outer.body();
    let middle = regions
        .iter()
        .find(|region| region.start() == middle_start)
        .ok_or("middle repeat")?;
    let inner_start = middle.body();
    let inner = regions
        .iter()
        .find(|region| region.start() == inner_start)
        .ok_or("inner repeat")?;
    let (signal, events) = drive_twice(&source, &[])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(5)));
    assert_eq!(start_count(&events, middle.start()), 2);
    assert_eq!(start_count(&events, inner.body()), 4);
    Ok(())
}

// Side effects do not leak across attempts: each attempt overwrites tick slot
// with the same entry value, not accumulated. The private result slot receives
// the body Copy per attempt and the final RepeatFinish forwarding.
#[test]
fn nested_repeat_side_effect_isolated_per_attempt() -> TestResult {
    let source = format!(
        "{HEADER}{INPUTS}steps:\n  - id: s\n    set: {{output: b, value: '1'}}\n  - id: o\n    for_each:\n      input: '$input.items'\n      variable: e\n      steps:\n        - id: i\n          repeat:\n            max_attempts: 3\n            steps:\n              - id: t\n                set: {{output: v, value: '$e'}}\n  - id: done\n    finish: {{result: '$b'}}\n"
    );
    let workflow = compile_workflow(source.as_bytes()).map_err(|e| format!("compile: {e:?}"))?;
    let regions = repeat_regions(&workflow)?;
    let region = regions.first().ok_or("repeat region")?;
    let (signal, events) = drive_twice(&source, &[11])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(1)));
    let tick_slot = workflow
        .node(region.body())
        .ok_or_else(|| "missing tick".to_owned())?
        .output
        .slot()
        .ok_or_else(|| "missing slot".to_owned())?;
    let finish = workflow.node(region.done()).ok_or("repeat finish")?;
    let CompiledNodeKind::RepeatFinish { result } = finish.kind else {
        return Err("repeat finish kind".to_owned());
    };
    assert_ne!(tick_slot, result);
    let writes = |target| {
        events
            .iter()
            .filter_map(|e| match e {
                EvidenceEvent::SlotWritten { slot, value, .. } if *slot == target => Some(*value),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let leaf_writes = writes(tick_slot);
    let result_writes = writes(result);
    assert_eq!(leaf_writes.len(), 3);
    assert!(leaf_writes.iter().all(|v| *v == SlotValue::I64(11)));
    assert_eq!(result_writes.len(), 4);
    assert!(result_writes.iter().all(|v| *v == SlotValue::I64(11)));
    Ok(())
}
