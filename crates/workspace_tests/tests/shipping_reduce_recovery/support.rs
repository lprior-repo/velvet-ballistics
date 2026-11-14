#![forbid(unsafe_code)]

//! Shared in-memory Reduce fixtures and bounded live/replay drivers.

use vb_compile::compile_workflow;
use vb_core::frame::RunFrameCheckpointV1;
use vb_core::replay::step::{ReplayAction, ReplayCollectStates, replay_step_with_collect};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{
    CapabilitySet, CompiledNodeKind, CompiledWorkflow, ListId, RunFrame, RunId, SlotIdx, SlotValue,
    StepBudget, StepIdx, Taint, ValueStore, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) const INPUT: u16 = 8;

pub(super) const SOURCE: &str = r#"version: velvet-ballistics/v1
name: shipping_reduce_recovery
when:
  manual: {}
steps:
  - id: folded
    reduce:
      variable: total
      input: "8"
      initial: "10"
      steps:
        - id: update
          set:
            output: local_value
            value: "$total + 1"
  - id: done
    finish:
      result: "$steps.folded"
"#;

pub(super) const TWO: &str = r#"version: velvet-ballistics/v1
name: shipping_reduce_two_activations
when:
  manual: {}
steps:
  - id: first
    reduce:
      variable: total
      input: "8"
      initial: "10"
      steps:
        - id: first_update
          set:
            output: first_local
            value: "$total + 1"
  - id: second
    reduce:
      variable: total
      input: "9"
      initial: "0"
      steps:
        - id: second_update
          set:
            output: second_local
            value: "$total + 1"
  - id: done
    finish:
      result: "$steps.second"
"#;

pub(super) fn compile(source: &str) -> TestResult<CompiledWorkflow> {
    Ok(compile_workflow(source.as_bytes())?)
}

pub(super) fn bind(
    workflow: &CompiledWorkflow,
    input: SlotIdx,
    values: &[SlotValue],
    taint: Taint,
    item_taints: &[Taint],
) -> TestResult<(RunFrame, ValueStore, ListId)> {
    let mut frame = new_run_frame(RunId::new(1), workflow)?;
    let mut store = ValueStore::new();
    let list = if item_taints.is_empty() {
        store.insert_list(values.to_vec().into_boxed_slice())?
    } else {
        assert_eq!(item_taints.len(), values.len(), "item taint arity");
        store.insert_list_with_taint(
            values.to_vec().into_boxed_slice(),
            item_taints.to_vec().into_boxed_slice(),
        )?
    };
    frame.write_slot_with_taint(input, SlotValue::List(list), taint)?;
    Ok((frame, store, list))
}

pub(super) fn drive(
    workflow: &CompiledWorkflow,
    frame: &mut RunFrame,
    store: &mut ValueStore,
    steps: u64,
    evidence: &mut EvidenceCollector,
) -> Result<RuntimeSignal, RuntimeEngineError> {
    drive_deterministic_full(
        workflow,
        frame,
        &mut StepBudget::new(steps),
        store,
        &[],
        RetryPolicy::NEVER,
        evidence,
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
}

pub(super) fn replay_one(
    workflow: &CompiledWorkflow,
    frame: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut ReplayCollectStates,
) -> TestResult<ReplayAction> {
    let node = workflow.node(frame.pc()).ok_or("replay node missing")?;
    replay_step_with_collect(node, frame, store, workflow, states)
        .map_err(|error| format!("replay step failed: {error:?}").into())
}

pub(super) fn replay_steps(
    workflow: &CompiledWorkflow,
    frame: &mut RunFrame,
    store: &mut ValueStore,
    steps: usize,
) -> TestResult {
    let mut states = ReplayCollectStates::new();
    for _ in 0..steps {
        match replay_one(workflow, frame, store, &mut states)? {
            ReplayAction::Continue(_) => {}
            ReplayAction::Finished => return Ok(()),
            _ => return Err("unexpected replay suspension".into()),
        }
    }
    Ok(())
}

pub(super) fn replay(
    workflow: &CompiledWorkflow,
    frame: &mut RunFrame,
    store: &mut ValueStore,
) -> TestResult {
    replay_steps(workflow, frame, store, 64)
}

pub(super) fn node_kind<F, T>(workflow: &CompiledWorkflow, pick: F, missing: &str) -> TestResult<T>
where
    F: Fn(&CompiledNodeKind) -> Option<T>,
{
    (0..=u16::MAX)
        .find_map(|raw| {
            workflow
                .node(StepIdx::new(raw))
                .and_then(|node| pick(&node.kind))
        })
        .ok_or_else(|| missing.into())
}

pub(super) fn accumulator(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    node_kind(
        workflow,
        |kind| match kind {
            CompiledNodeKind::ReduceStart { accumulator, .. } => Some(*accumulator),
            _ => None,
        },
        "ReduceStart absent",
    )
}

pub(super) struct ReduceNodes {
    pub(super) start: StepIdx,
    pub(super) body: StepIdx,
    pub(super) next: StepIdx,
    pub(super) finish: StepIdx,
    pub(super) done: StepIdx,
}

pub(super) fn reduce_nodes(workflow: &CompiledWorkflow) -> TestResult<ReduceNodes> {
    let mut start = None;
    let mut next = None;
    let mut finish = None;
    let mut done = None;
    for raw in 0..=u16::MAX {
        let Some(node) = workflow.node(StepIdx::new(raw)) else {
            continue;
        };
        match node.kind {
            CompiledNodeKind::ReduceStart { body, .. } => start = Some((node.id, body)),
            CompiledNodeKind::ReduceNext { .. } => next = Some(node.id),
            CompiledNodeKind::ReduceFinish { .. } => finish = Some(node.id),
            CompiledNodeKind::Finish { .. } => done = Some(node.id),
            _ => {}
        }
    }
    let (start, body) = start.ok_or("ReduceStart absent")?;
    Ok(ReduceNodes {
        start,
        body,
        next: next.ok_or("ReduceNext absent")?,
        finish: finish.ok_or("ReduceFinish absent")?,
        done: done.ok_or("Finish absent")?,
    })
}

pub(super) fn reduce_input_slot(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    node_kind(
        workflow,
        |kind| match kind {
            CompiledNodeKind::ReduceStart { input, .. } => Some(*input),
            _ => None,
        },
        "ReduceStart absent",
    )
}

pub(super) fn reduce_next_item_slot(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    (0..=u16::MAX)
        .find_map(|raw| {
            let node = workflow.node(StepIdx::new(raw))?;
            match node.kind {
                CompiledNodeKind::ReduceNext { .. } => node.output.slot(),
                _ => None,
            }
        })
        .ok_or_else(|| "ReduceNext output absent".into())
}

pub(super) fn iterator_slot(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    node_kind(
        workflow,
        |kind| match kind {
            CompiledNodeKind::ReduceNext { iterator_slot, .. } => Some(*iterator_slot),
            _ => None,
        },
        "ReduceNext absent",
    )
}

pub(super) fn finish_result(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    node_kind(
        workflow,
        |kind| match kind {
            CompiledNodeKind::Finish { result } => Some(*result),
            _ => None,
        },
        "Finish absent",
    )
}

pub(super) fn slots(frame: &RunFrame) -> TestResult<Vec<(SlotIdx, SlotValue, Taint)>> {
    let mut entries = frame.initialized_slots()?;
    entries.sort_by_key(|entry| entry.0.get());
    Ok(entries)
}

pub(super) fn restore_frame(checkpoint: RunFrameCheckpointV1) -> TestResult<RunFrame> {
    RunFrame::from_checkpoint_v1(checkpoint)
        .map_err(|error| format!("frame checkpoint restore failed: {error:?}").into())
}

pub(super) fn restore_store(checkpoint: ValueStoreCheckpointV1) -> TestResult<ValueStore> {
    ValueStore::from_checkpoint_v1(checkpoint)
        .map_err(|error| format!("store checkpoint restore failed: {error:?}").into())
}

pub(super) fn assert_source(
    frame: &RunFrame,
    store: &ValueStore,
    list: ListId,
    values: &[SlotValue],
    taint: Taint,
) {
    assert_eq!(
        frame.read_slot(SlotIdx::new(INPUT)),
        Ok(&SlotValue::List(list))
    );
    assert_eq!(frame.read_taint(SlotIdx::new(INPUT)), Ok(taint));
    assert_eq!(store.list(list), Ok(values));
}
