#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::replay::ReplayError;
use vb_core::replay::step::ReplayCollectStates;
use vb_core::replay::step::replay_step_with_collect;
use vb_core::{
    CapabilitySet, ListId, RunId, SlotIdx, SlotValue, StepBudget, StepIdx, Taint, ValueStore,
    new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, RuntimeEngineError, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const SOURCE_OVERFLOW: &str = r#"version: velvet-ballistics/v1
name: reduce_overflow
when:
  manual: {}
steps:
  - id: reduce_start
    reduce:
      variable: total
      input: "8"
      initial: "9223372036854775807"
      steps:
        - id: body
          set:
            output: next_total
            value: "$total + 1"
  - id: done
    finish:
      result: "$steps.reduce_start"
"#;

const SOURCE_DIVIDE_BY_ZERO: &str = r#"version: velvet-ballistics/v1
name: reduce_div_zero
when:
  manual: {}
steps:
  - id: reduce_start
    reduce:
      variable: total
      input: "8"
      initial: "10"
      steps:
        - id: body
          set:
            output: next_total
            value: "$total / 0"
  - id: done
    finish:
      result: "$steps.reduce_start"
"#;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn compile_source(source: &str) -> TestResult<vb_core::CompiledWorkflow> {
    Ok(compile_workflow(source.as_bytes())?)
}

fn original_ir(workflow: &vb_core::CompiledWorkflow) -> TestResult<Vec<u8>> {
    Ok(postcard::to_allocvec(&workflow.to_parts())?)
}

fn derive_body_expr_step_idx(workflow: &vb_core::CompiledWorkflow) -> TestResult<StepIdx> {
    let parts = workflow.to_parts();
    for node in &parts.nodes {
        if let vb_core::CompiledNodeKind::ReduceStart { body, .. } = &node.kind {
            return Ok(*body);
        }
    }
    Err("no ReduceStart node found in compiled workflow".into())
}

fn assert_node_is_eval_expr(workflow: &vb_core::CompiledWorkflow, step: StepIdx) -> TestResult<()> {
    let node = workflow
        .node(step)
        .ok_or("node at derived body step is None")?;
    if !matches!(node.kind, vb_core::CompiledNodeKind::EvalExpr { .. }) {
        return Err(format!(
            "node at derived body step {:?} should be EvalExpr, got {:?}",
            step, node.kind
        )
        .into());
    }
    Ok(())
}

fn make_frame(workflow: &vb_core::CompiledWorkflow) -> TestResult<vb_core::RunFrame> {
    Ok(new_run_frame(RunId::new(1), workflow)?)
}

fn bind_singleton_list(
    frame: &mut vb_core::RunFrame,
    store: &mut ValueStore,
) -> TestResult<ListId> {
    let values = vec![SlotValue::I64(4)];
    let list = store.insert_list(values)?;
    frame.write_slot_with_taint(
        SlotIdx::new(8),
        SlotValue::List(list),
        Taint::DerivedFromSecret,
    )?;
    Ok(list)
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

fn run_replay_step(
    workflow: &vb_core::CompiledWorkflow,
    frame: &mut vb_core::RunFrame,
    store: &mut ValueStore,
) -> Result<vb_core::replay::step::ReplayAction, ReplayError> {
    let mut states = ReplayCollectStates::new();
    for _ in 0..4 {
        let pc = frame.pc();
        let node = workflow.node(pc).ok_or(ReplayError::Internal {
            reason: "replay node missing",
        })?;
        match replay_step_with_collect(node, frame, store, workflow, &mut states)? {
            vb_core::replay::step::ReplayAction::Continue(_) => {}
            other => return Ok(other),
        }
    }
    Err(ReplayError::Internal {
        reason: "test replay budget exhausted",
    })
}

fn assert_failed_state(
    frame: &vb_core::RunFrame,
    list: ListId,
    store: &ValueStore,
    original: &[u8],
    workflow: &vb_core::CompiledWorkflow,
) -> TestResult<()> {
    assert_eq!(*frame.read_slot(SlotIdx::new(8))?, SlotValue::List(list));
    assert_eq!(frame.read_taint(SlotIdx::new(8))?, Taint::DerivedFromSecret);
    let preserved = store.list(list)?;
    assert_eq!(preserved, &[SlotValue::I64(4)]);
    let after = postcard::to_allocvec(&workflow.to_parts())?;
    assert_eq!(original, &after);
    Ok(())
}

fn run_live_path(
    workflow: &vb_core::CompiledWorkflow,
    body_step: StepIdx,
    expected: RuntimeEngineError,
    original: &[u8],
) -> TestResult<()> {
    let mut frame = make_frame(workflow)?;
    let mut store = ValueStore::new();
    let list = bind_singleton_list(&mut frame, &mut store)?;
    let result = live_drive(workflow, &mut frame, &mut store);
    match result {
        Ok(sig) => return Err(format!("live should fail, got {sig:?}").into()),
        Err(e) if e == expected => {}
        Err(e) => return Err(format!("wrong live error: {e:?}").into()),
    }
    assert_eq!(frame.pc(), body_step, "live PC must be at body expression");
    assert_failed_state(&frame, list, &store, original, workflow)?;
    Ok(())
}

fn run_replay_path(
    workflow: &vb_core::CompiledWorkflow,
    body_step: StepIdx,
    expected: ReplayError,
    original: &[u8],
) -> TestResult<()> {
    let mut frame = make_frame(workflow)?;
    let mut store = ValueStore::new();
    let list = bind_singleton_list(&mut frame, &mut store)?;
    let result = run_replay_step(workflow, &mut frame, &mut store);
    match result {
        Err(e) if e == expected => {}
        Err(e) => return Err(format!("wrong replay error: {e:?}").into()),
        Ok(_action) => {
            return Err("replay should fail, got Ok(action)".into());
        }
    }
    assert_eq!(
        frame.pc(),
        body_step,
        "replay PC must be at body expression"
    );
    assert_failed_state(&frame, list, &store, original, workflow)?;
    Ok(())
}

fn run_arithmetic_error_cases(source: &str, expected_live: RuntimeEngineError) -> TestResult<()> {
    let workflow = compile_source(source)?;
    let original = original_ir(&workflow)?;
    let body_step = derive_body_expr_step_idx(&workflow)?;
    assert_node_is_eval_expr(&workflow, body_step)?;

    let live_expected = expected_live.clone();
    run_live_path(&workflow, body_step, live_expected, &original)?;

    let replay_expected = ReplayError::ExpressionEvalFailed { step: body_step };
    run_replay_path(&workflow, body_step, replay_expected, &original)?;

    Ok(())
}

#[test]
fn reduce_overflow_live_and_replay_error() -> TestResult {
    let expected_live = RuntimeEngineError::Core(vb_core::EngineError::InvalidCompiledWorkflow {
        reason: "integer arithmetic overflow",
    });
    run_arithmetic_error_cases(SOURCE_OVERFLOW, expected_live)
}

#[test]
fn reduce_divide_by_zero_live_and_replay_error() -> TestResult {
    let expected_live = RuntimeEngineError::Core(vb_core::EngineError::DivisionByZero);
    run_arithmetic_error_cases(SOURCE_DIVIDE_BY_ZERO, expected_live)
}
