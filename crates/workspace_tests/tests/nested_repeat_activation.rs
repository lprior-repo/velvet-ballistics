//! Nested repeat activation: repeated activation, check exhaustion, and
//! per-attempt rearm via source-driven drive.
//!
//! Bead: vb-eirmk.4.5. Proves packed counter ownership across nested repeats
//! and outer loops, plus check/done routes under repeated activation.

#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::{CapabilitySet, RunId, SlotValue, StepBudget, Taint, ValueStore, new_run_frame};
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

fn start_count(events: &[EvidenceEvent], step: u16) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepStarted { step: s } if s.get() == step))
        .count()
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
    let (signal, events) = drive_twice(&source, &[])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(10)));
    assert_eq!(start_count(&events, 2), 2);
    assert_eq!(start_count(&events, 3), 2);
    assert_eq!(start_count(&events, 4), 1);
    Ok(())
}

// Check exhaustion: max 1 runs once, check immediately exhausts to done.
#[test]
fn repeat_check_exhaustion_routes_to_done() -> TestResult {
    let source = flat_repeat_source(1);
    let (signal, events) = drive_twice(&source, &[])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(10)));
    assert_eq!(start_count(&events, 2), 1);
    assert_eq!(start_count(&events, 3), 1);
    assert_eq!(start_count(&events, 4), 1);
    Ok(())
}

// Repeated activation: nested repeat under for_each rearms per outer item.
// Outer 3 items, inner max 2 => tick runs 3*2=6 times.
#[test]
fn nested_repeat_repeated_activation_resets_attempt_counter() -> TestResult {
    let source = format!(
        "{HEADER}{INPUTS}steps:\n  - id: s\n    set: {{output: b, value: '99'}}\n  - id: o\n    for_each:\n      input: '$input.items'\n      variable: e\n      steps:\n        - id: i\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: t\n                set: {{output: v, value: '$e'}}\n  - id: done\n    finish: {{result: '$b'}}\n"
    );
    let (signal, events) = drive_twice(&source, &[7, 8, 9])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(99)));
    assert_eq!(start_count(&events, 1), 1);
    assert_eq!(start_count(&events, 2), 3);
    assert_eq!(start_count(&events, 3), 6);
    assert_eq!(start_count(&events, 4), 6);
    assert_eq!(start_count(&events, 6), 3);
    Ok(())
}

// Triple nesting: repeat outer drives each inner activation to exhaustion.
// Verifies per-attempt rearm for multiple levels.
#[test]
fn triple_nested_repeat_per_attempt_rearm() -> TestResult {
    let source = format!(
        "{HEADER}steps:\n  - id: s\n    set: {{output: b, value: '5'}}\n  - id: o\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: m\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: i\n                repeat:\n                  max_attempts: 1\n                  steps:\n                    - id: leaf\n                      set: {{output: v, value: '1'}}\n  - id: done\n    finish: {{result: '$b'}}\n"
    );
    let (signal, events) = drive_twice(&source, &[])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(5)));
    // Layout: 1 outer,2 mid-start,3 inner-start,4 leaf. Mid start runs once
    // per outer entry (2), leaf runs outer*mid*inner = 2*2*1=4.
    assert_eq!(start_count(&events, 2), 2);
    assert_eq!(start_count(&events, 4), 4);
    Ok(())
}

// Side effects do not leak across attempts: each attempt overwrites tick slot
// with the same entry value, not accumulated. RepeatFinish forwards the
// result once more, so tick slot sees 3 body writes + 1 finish forwarding.
#[test]
fn nested_repeat_side_effect_isolated_per_attempt() -> TestResult {
    let source = format!(
        "{HEADER}{INPUTS}steps:\n  - id: s\n    set: {{output: b, value: '1'}}\n  - id: o\n    for_each:\n      input: '$input.items'\n      variable: e\n      steps:\n        - id: i\n          repeat:\n            max_attempts: 3\n            steps:\n              - id: t\n                set: {{output: v, value: '$e'}}\n  - id: done\n    finish: {{result: '$b'}}\n"
    );
    let workflow = compile_workflow(source.as_bytes()).map_err(|e| format!("compile: {e:?}"))?;
    let (signal, events) = drive_twice(&source, &[11])?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(1)));
    let tick_slot = workflow
        .node(vb_core::StepIdx::new(3))
        .ok_or_else(|| "missing tick".to_owned())?
        .output
        .slot()
        .ok_or_else(|| "missing slot".to_owned())?;
    let writes: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::SlotWritten { slot, value, .. } if *slot == tick_slot => Some(*value),
            _ => None,
        })
        .collect();
    // tick runs 3 times per outer item (1 item) => 3 body writes + 1 finish forward
    assert_eq!(writes.len(), 4);
    assert!(writes.iter().all(|v| *v == SlotValue::I64(11)));
    Ok(())
}
