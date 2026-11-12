#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::replay::step::{ReplayAction, ReplayCollectStates, replay_step_with_collect};
use vb_core::{
    CapabilitySet, CompiledNodeKind, ListId, RunId, SlotIdx, SlotValue, StepBudget, Taint,
    ValueStore, new_run_frame,
};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SOURCE: &str = "version: velvet-ballistics/v1\nname: reduce_replay_parity\nwhen:\n  manual: {}\nsteps:\n  - id: folded\n    reduce:\n      variable: total\n      input: \"8\"\n      initial: \"10\"\n      steps:\n        - id: update\n          set:\n            output: local_value\n            value: \"$total + 1\"\n  - id: done\n    finish:\n      result: \"$steps.folded\"\n";

fn compile() -> TestResult<vb_core::CompiledWorkflow> {
    Ok(compile_workflow(SOURCE.as_bytes())?)
}

fn make_frame(workflow: &vb_core::CompiledWorkflow) -> TestResult<vb_core::RunFrame> {
    Ok(new_run_frame(RunId::new(1), workflow)?)
}

fn bind_input(
    frame: vb_core::RunFrame,
    values: &[SlotValue],
) -> TestResult<(vb_core::RunFrame, ValueStore, ListId)> {
    let mut store = ValueStore::new();
    let input_list = store.insert_list(values.to_vec())?;
    let mut frame = frame;
    frame.write_slot_with_taint(
        SlotIdx::new(8),
        SlotValue::List(input_list),
        Taint::DerivedFromSecret,
    )?;
    Ok((frame, store, input_list))
}

fn values_from_i64(input: &[i64]) -> Vec<SlotValue> {
    input.iter().copied().map(SlotValue::I64).collect()
}

fn live_signal(
    workflow: &vb_core::CompiledWorkflow,
    frame: &mut vb_core::RunFrame,
    store: &mut ValueStore,
) -> TestResult<RuntimeSignal> {
    drive_deterministic_full(
        workflow,
        frame,
        &mut StepBudget::new(128),
        store,
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
    .map_err(Into::into)
}

fn check_terminal(
    frame: &vb_core::RunFrame,
    workflow: &vb_core::CompiledWorkflow,
    expected: i64,
) -> TestResult {
    let pc = frame.pc();
    let node = workflow.node(pc).ok_or("terminal node missing")?;
    let CompiledNodeKind::Finish { result } = &node.kind else {
        return Err("terminal node not Finish".into());
    };
    assert_eq!(frame.read_slot(*result), Ok(&SlotValue::I64(expected)));
    assert_eq!(frame.read_taint(*result), Ok(Taint::Clean));
    Ok(())
}

fn assert_source_preserved(
    frame: &vb_core::RunFrame,
    store: &ValueStore,
    input_list: ListId,
    input: &[SlotValue],
) {
    assert_eq!(
        frame.read_slot(SlotIdx::new(8)),
        Ok(&SlotValue::List(input_list))
    );
    assert_eq!(store.list(input_list), Ok(input));
    assert_eq!(
        frame.read_taint(SlotIdx::new(8)),
        Ok(Taint::DerivedFromSecret)
    );
}

fn replay_to_finish(
    workflow: &vb_core::CompiledWorkflow,
    frame: &mut vb_core::RunFrame,
    store: &mut ValueStore,
) -> TestResult {
    let mut states = ReplayCollectStates::new();
    for _ in 0..64 {
        let pc = frame.pc();
        let node = workflow.node(pc).ok_or("node missing at pc")?;
        let action = replay_step_with_collect(node, frame, store, workflow, &mut states).map_err(
            |e| -> Box<dyn std::error::Error> { format!("replay step failed: {e:?}").into() },
        )?;
        match action {
            ReplayAction::Continue(_) => {}
            ReplayAction::Finished => return Ok(()),
            _other => return Err(format!("unexpected replay action at {pc:?}").into()),
        }
    }
    Err("replay exceeded step budget".into())
}

fn run_parity(input: &[i64], expected: i64) -> TestResult {
    let workflow = compile()?;
    let values = values_from_i64(input);

    let frame = make_frame(&workflow)?;
    let (mut live_frame, mut live_store, live_input_list) = bind_input(frame, &values)?;
    let signal = live_signal(&workflow, &mut live_frame, &mut live_store)?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(expected)));
    check_terminal(&live_frame, &workflow, expected)?;
    assert_source_preserved(&live_frame, &live_store, live_input_list, &values);

    let frame = make_frame(&workflow)?;
    let (mut replay_frame, mut replay_store, replay_input_list) = bind_input(frame, &values)?;
    replay_to_finish(&workflow, &mut replay_frame, &mut replay_store)?;
    check_terminal(&replay_frame, &workflow, expected)?;
    assert_source_preserved(&replay_frame, &replay_store, replay_input_list, &values);

    Ok(())
}

#[test]
fn reduce_replay_empty_input_returns_initial() -> TestResult {
    run_parity(&[], 10)
}

#[test]
fn reduce_replay_single_input_increments_once() -> TestResult {
    run_parity(&[4], 11)
}

#[test]
fn reduce_replay_multiple_inputs_increments_per_item() -> TestResult {
    run_parity(&[4, 7, -3], 13)
}
