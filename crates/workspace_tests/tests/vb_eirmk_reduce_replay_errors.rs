#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::replay::ReplayError;
use vb_core::replay::step::{ReplayAction, ReplayCollectStates, replay_step_with_collect};
use vb_core::{
    CapabilitySet, ListId, RunId, SlotIdx, SlotValue, StepBudget, StepIdx, Taint, ValueStore,
    new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, RuntimeEngineError, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const SOURCE: &str = r#"version: velvet-ballistics/v1
name: reduce_error
when:
  manual: {}
steps:
  - id: reduce_start
    reduce:
      variable: total
      input: "8"
      initial: "0"
      steps:
        - id: body
          set:
            output: next_total
            value: "$total + 1"
  - id: done
    finish:
      result: "$steps.reduce_start"
"#;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn compile_source() -> TestResult<vb_core::CompiledWorkflow> {
    Ok(compile_workflow(SOURCE.as_bytes())?)
}

fn original_ir(workflow: &vb_core::CompiledWorkflow) -> TestResult<Vec<u8>> {
    Ok(postcard::to_allocvec(&workflow.to_parts())?)
}

fn make_frame(
    workflow: &vb_core::CompiledWorkflow,
    value: SlotValue,
) -> TestResult<vb_core::RunFrame> {
    let mut frame = new_run_frame(RunId::new(1), workflow)?;
    frame.write_slot_with_taint(SlotIdx::new(8), value, Taint::DerivedFromSecret)?;
    Ok(frame)
}

fn live_drive(
    workflow: &vb_core::CompiledWorkflow,
    frame: &mut vb_core::RunFrame,
    store: &mut ValueStore,
) -> Result<RuntimeSignal, RuntimeEngineError> {
    drive_deterministic_full(
        workflow,
        frame,
        &mut StepBudget::new(64),
        store,
        &[],
        vb_runtime::engine::RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
}

fn run_replay(
    workflow: &vb_core::CompiledWorkflow,
    frame: &mut vb_core::RunFrame,
    store: &mut ValueStore,
) -> Result<ReplayAction, ReplayError> {
    let mut states = ReplayCollectStates::new();
    for _ in 0..8 {
        let pc = frame.pc();
        let node = workflow.node(pc).ok_or(ReplayError::Internal {
            reason: "replay node missing",
        })?;
        match replay_step_with_collect(node, frame, store, workflow, &mut states)? {
            ReplayAction::Continue(_) => {}
            other => return Ok(other),
        }
    }
    Err(ReplayError::Internal {
        reason: "test replay budget exhausted",
    })
}

fn assert_failed_state(
    frame: &vb_core::RunFrame,
    value: &SlotValue,
    store: &ValueStore,
    original: &[u8],
    workflow: &vb_core::CompiledWorkflow,
) -> TestResult<()> {
    let pc = frame.pc();
    assert_eq!(pc, StepIdx::new(1));
    assert_eq!(frame.read_slot(SlotIdx::new(8))?, value);
    assert_eq!(frame.read_taint(SlotIdx::new(8))?, Taint::DerivedFromSecret);
    assert_eq!(store.list_count(), 0);
    assert_eq!(store, &ValueStore::new());
    let after = postcard::to_allocvec(&workflow.to_parts())?;
    assert_eq!(original, &after);
    let node = workflow.node(pc).ok_or("node at failure pc is None")?;
    if !matches!(node.kind, vb_core::CompiledNodeKind::ReduceStart { .. }) {
        return Err("node at failure pc should be ReduceStart".into());
    }
    Ok(())
}

fn run_live_path(
    workflow: &vb_core::CompiledWorkflow,
    value: SlotValue,
    expected: RuntimeEngineError,
    original: &[u8],
) -> TestResult<()> {
    let mut frame = make_frame(workflow, value)?;
    let mut store = ValueStore::new();
    let result = live_drive(workflow, &mut frame, &mut store);
    match result {
        Ok(sig) => return Err(format!("live should fail, got {sig:?}").into()),
        Err(e) if e == expected => {}
        Err(e) => return Err(format!("wrong live error: {e:?}").into()),
    }
    assert_failed_state(&frame, &value, &store, original, workflow)?;
    Ok(())
}

fn run_replay_path(
    workflow: &vb_core::CompiledWorkflow,
    value: SlotValue,
    expected: ReplayError,
    original: &[u8],
) -> TestResult<()> {
    let mut frame = make_frame(workflow, value)?;
    let mut store = ValueStore::new();
    let result = run_replay(workflow, &mut frame, &mut store);
    match result {
        Err(e) if e == expected => {}
        Err(e) => return Err(format!("wrong replay error: {e:?}").into()),
        Ok(_action) => return Err("replay should fail, got Ok(action)".into()),
    }
    assert_failed_state(&frame, &value, &store, original, workflow)?;
    Ok(())
}

fn check_type_mismatch(value: SlotValue, found: &'static str) -> TestResult<()> {
    let workflow = compile_source()?;
    let original = original_ir(&workflow)?;
    let expected_live = RuntimeEngineError::Core(vb_core::EngineError::TypeMismatch {
        expected: "list",
        found,
    });
    let expected_replay = ReplayError::Internal {
        reason: "loop slot was not list during replay",
    };
    run_live_path(&workflow, value, expected_live, &original)?;
    run_replay_path(&workflow, value, expected_replay, &original)?;
    Ok(())
}

fn check_dangling_list() -> TestResult<()> {
    let workflow = compile_source()?;
    let original = original_ir(&workflow)?;
    let dangling = ListId::new(u32::MAX);
    let value = SlotValue::List(dangling);
    let expected_live =
        RuntimeEngineError::Core(vb_core::EngineError::ListOutOfBounds { list: dangling });
    let expected_replay = ReplayError::Internal {
        reason: "reduce source list missing during replay",
    };
    run_live_path(&workflow, value, expected_live, &original)?;
    run_replay_path(&workflow, value, expected_replay, &original)?;
    Ok(())
}

#[test]
fn reduce_live_and_replay_fail_on_wrong_type_i64() -> TestResult {
    check_type_mismatch(SlotValue::I64(42), "number")
}

#[test]
fn reduce_live_and_replay_fail_on_wrong_type_bool() -> TestResult {
    check_type_mismatch(SlotValue::Bool(true), "boolean")
}

#[test]
fn reduce_live_and_replay_fail_on_dangling_list_id() -> TestResult {
    check_dangling_list()
}
