#![forbid(unsafe_code)]
//! Source-driven regressions for owned-region rearm (bead vb-8eocj.3.7).
//! All workflows are compiled from YAML via `compile_workflow`; no hand IR.

use vb_compile::compile_workflow;
use vb_core::engine::EngineError;
use vb_core::{
    CapabilitySet, CompiledNodeKind, CompiledWorkflow, ConstValue, RunFrame, RunId, SlotIdx,
    SlotValue, StepBudget, StepIdx, Taint, ValueStore,
    frame::{StepState, is_valid_step_state_transition},
    new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal,
    drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const HEADER: &str = "version: velvet-ballistics/v1\nname: loop_rearm_source\nwhen: {manual: {}}\n";
const INPUTS: &str = "inputs:\n  items: {is: list, of: any, max: 4}\n";
type TestResult = Result<(), String>;

fn compile(source: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source.as_bytes()).map_err(|e| format!("compile: {e:?}"))
}

fn frame(workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
    new_run_frame(RunId::new(1), workflow).map_err(|e| format!("frame: {e:?}"))
}

fn aerr<T, E: core::fmt::Debug>(r: Result<T, E>) -> Result<T, String> {
    r.map_err(|e| format!("{e:?}"))
}

fn repeat_source(max: u64) -> String {
    format!(
        "{HEADER}steps:\n  - id: first\n    set:\n      output: answer\n      value: \"42\"\n  - id: second\n    set:\n      output: sentinel\n      value: \"99\"\n  - id: bounded_repeat\n    repeat:\n      max_attempts: {max}\n      steps:\n        - id: body_first\n          set:\n            output: body_first_result\n            value: \"7\"\n        - id: body_second\n          set:\n            output: body_second_result\n            value: \"11\"\n  - id: done\n    finish:\n      result: answer\n"
    )
}

fn body_values(wf: &CompiledWorkflow, events: &[EvidenceEvent]) -> Vec<i64> {
    let mut out = Vec::new();
    for e in events {
        if let EvidenceEvent::StepSucceeded { step, .. } = e {
            if let Some(node) = wf.node(*step) {
                if let CompiledNodeKind::SetConst { value } = &node.kind {
                    if let Some(&ConstValue::I64(v)) = wf.constant(*value) {
                        if v == 7 || v == 11 {
                            out.push(v);
                        }
                    }
                }
            }
        }
    }
    out
}

fn control_body(wf: &CompiledWorkflow) -> Result<(StepIdx, StepIdx, SlotIdx), String> {
    for i in 0..wf.node_count() {
        let id = StepIdx::new(i);
        if let Some(node) = wf.node(id) {
            if let CompiledNodeKind::RepeatCheck { attempt_slot, .. } = &node.kind {
                let body = node
                    .routing
                    .next
                    .step()
                    .ok_or_else(|| "repeat check missing body".to_owned())?;
                return Ok((id, body, *attempt_slot));
            }
        }
    }
    Err("missing RepeatCheck".to_owned())
}

fn state(run: &RunFrame, id: StepIdx) -> Result<StepState, String> {
    aerr(run.step_state(id))
}

#[test]
fn multistep_repeat_rearms_all_body_steps() -> TestResult {
    let wf = compile(&repeat_source(3))?;
    let mut run = frame(&wf)?;
    let mut store = ValueStore::new();
    let mut ev = EvidenceCollector::new();
    let mut cs = CollectStates::new();
    let sig = drive_deterministic_full(
        &wf,
        &mut run,
        &mut StepBudget::new(128),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut ev,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|e| format!("drive: {e:?}"))?;
    assert_eq!(sig, RuntimeSignal::Finished(SlotValue::I64(42)));
    let vals = body_values(&wf, &ev.drain());
    assert_eq!(vals, vec![7, 11, 7, 11, 7, 11]);
    Ok(())
}

fn nested_source() -> String {
    format!(
        "{HEADER}{INPUTS}steps:\n  - id: seed\n    set: {{output: base, value: '100'}}\n  - id: outer\n    for_each:\n      input: '$input.items'\n      variable: entry\n      steps:\n        - id: inner\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: ha\n                set: {{output: va, value: '$entry'}}\n              - id: hb\n                set: {{output: vb, value: '77'}}\n        - id: grab\n          set: {{output: grabbed, value: '$step.inner'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

fn drive_listed(
    wf: &CompiledWorkflow,
    items: &[i64],
) -> Result<(RuntimeSignal, Vec<EvidenceEvent>), String> {
    let mut run = frame(wf)?;
    let mut store = ValueStore::new();
    let vals: Vec<SlotValue> = items.iter().copied().map(SlotValue::I64).collect();
    let list = aerr(store.insert_list(vals))?;
    let slot = wf
        .input_slots()
        .first()
        .ok_or_else(|| "missing input slot".to_owned())?
        .slot;
    aerr(run.write_slot_with_taint(slot, SlotValue::List(list), Taint::Secret))?;
    let mut ev = EvidenceCollector::new();
    let mut cs = CollectStates::new();
    let sig = drive_deterministic_full(
        wf,
        &mut run,
        &mut StepBudget::new(256),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut ev,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|e| format!("drive: {e:?}"))?;
    Ok((sig, ev.drain()))
}

fn starts(events: &[EvidenceEvent], id: StepIdx) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepStarted { step } if *step == id))
        .count()
}

#[test]
fn nested_source_loops_rearm_both_levels() -> TestResult {
    let src = nested_source();
    let wf = compile(&src)?;
    let (sig, ev) = drive_listed(&wf, &[5, 6])?;
    assert_eq!(sig, RuntimeSignal::Finished(SlotValue::I64(100)));
    let inner_check = (0..wf.node_count())
        .map(StepIdx::new)
        .find(|id| {
            wf.node(*id)
                .is_some_and(|n| matches!(n.kind, CompiledNodeKind::RepeatCheck { .. }))
        })
        .ok_or_else(|| "missing inner check".to_owned())?;
    let outer_next = (0..wf.node_count())
        .map(StepIdx::new)
        .find(|id| {
            wf.node(*id)
                .is_some_and(|n| matches!(n.kind, CompiledNodeKind::ForEachNext { .. }))
        })
        .ok_or_else(|| "missing outer next".to_owned())?;
    assert_eq!(starts(&ev, inner_check), 4);
    assert_eq!(starts(&ev, outer_next), 2);
    Ok(())
}

#[test]
fn outer_state_preserved_across_rearm() -> TestResult {
    let wf = compile(&repeat_source(3))?;
    let mut run = frame(&wf)?;
    let mut store = ValueStore::new();
    let mut ev = EvidenceCollector::new();
    let mut cs = CollectStates::new();
    let sig = drive_deterministic_full(
        &wf,
        &mut run,
        &mut StepBudget::new(128),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut ev,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|e| format!("drive: {e:?}"))?;
    assert_eq!(sig, RuntimeSignal::Finished(SlotValue::I64(42)));
    let (control, body, _) = control_body(&wf)?;
    assert_eq!(state(&run, StepIdx::new(0))?, StepState::Succeeded);
    assert_eq!(state(&run, StepIdx::new(1))?, StepState::Succeeded);
    assert_eq!(state(&run, body)?, StepState::Succeeded);
    assert_eq!(state(&run, control)?, StepState::Succeeded);
    let slot = wf
        .node(StepIdx::new(0))
        .and_then(|n| n.output.slot())
        .ok_or_else(|| "missing outer output".to_owned())?;
    assert_eq!(aerr(run.read_slot(slot))?, &SlotValue::I64(42));
    assert_eq!(aerr(run.read_taint(slot))?, Taint::Clean);
    Ok(())
}

// White-box fixture exemption: the workflow is still source-compiled (no
// hand IR); only the `RunFrame` state is hand-driven below to reach in-flight
// mid-body state that source driving alone cannot reach.
#[test]
fn inflight_body_rejects_rearm_source() -> TestResult {
    let wf = compile(&repeat_source(3))?;
    let (control, body, attempt) = control_body(&wf)?;
    let mut run = frame(&wf)?;
    aerr(run.set_pc(control))?;
    let packed = i64::from(3).checked_shl(32).ok_or("pack overflow")?;
    aerr(run.write_slot_with_taint(attempt, SlotValue::I64(packed), Taint::Clean))?;
    aerr(run.mark_running(body))?;
    aerr(run.mark_succeeded(body))?;
    let later = StepIdx::new(body.get().checked_add(1).ok_or("overflow")?);
    aerr(run.mark_running(later))?;
    aerr(run.mark_waiting(later))?;
    let mut store = ValueStore::new();
    let mut ev = EvidenceCollector::new();
    let mut cs = CollectStates::new();
    let r = drive_deterministic_full(
        &wf,
        &mut run,
        &mut StepBudget::new(1),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut ev,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    match r {
        Err(RuntimeEngineError::Core(EngineError::InvalidCompiledWorkflow { reason }))
            if reason == "loop body step not in rearmable state" => {}
        other => return Err(format!("unexpected rearm outcome: {other:?}")),
    }
    assert_eq!(state(&run, body)?, StepState::Succeeded);
    assert_eq!(state(&run, later)?, StepState::Waiting);
    Ok(())
}

#[test]
fn no_global_relaxation_preserved() -> TestResult {
    assert!(is_valid_step_state_transition(
        StepState::Succeeded,
        StepState::Pending
    ));
    assert!(is_valid_step_state_transition(
        StepState::Pending,
        StepState::Pending
    ));
    for s in [
        StepState::Running,
        StepState::Failed,
        StepState::Skipped,
        StepState::Waiting,
        StepState::Asking,
        StepState::Cancelled,
    ] {
        assert!(
            !is_valid_step_state_transition(s, StepState::Pending),
            "{s:?}->Pending"
        );
    }
    let wf = compile(&repeat_source(1))?;
    let (control, body, _) = control_body(&wf)?;
    let mut run = frame(&wf)?;
    let mut store = ValueStore::new();
    let mut ev = EvidenceCollector::new();
    let mut cs = CollectStates::new();
    let sig = drive_deterministic_full(
        &wf,
        &mut run,
        &mut StepBudget::new(64),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut ev,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|e| format!("drive: {e:?}"))?;
    assert_eq!(sig, RuntimeSignal::Finished(SlotValue::I64(42)));
    assert_eq!(state(&run, body)?, StepState::Succeeded);
    assert_eq!(state(&run, control)?, StepState::Succeeded);
    Ok(())
}
