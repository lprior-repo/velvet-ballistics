#![forbid(unsafe_code)]

//! Reduce replay accumulator/termination parity, cold reopen, taints, and typed refusals.

use vb_compile::compile_workflow;
use vb_core::replay::ReplayError;
use vb_core::replay::step::{ReplayAction, ReplayCollectStates, replay_step_with_collect};
use vb_core::{
    CapabilitySet, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ListId, RunFrame,
    RunId, SlotIdx, SlotValue, StepBudget, StepIdx, Taint, ValueStore, WorkflowDigest,
    WorkflowParts, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal,
    drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;
use vb_runtime::primitives::reduce::{reduce_next, reduce_start};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const INPUT: u16 = 8;

const SOURCE: &str = r#"version: velvet-ballistics/v1
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

const TWO: &str = r#"version: velvet-ballistics/v1
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

fn compile(source: &str) -> TestResult<CompiledWorkflow> {
    Ok(compile_workflow(source.as_bytes())?)
}

fn bind(
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

fn drive(
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

fn replay(workflow: &CompiledWorkflow, frame: &mut RunFrame, store: &mut ValueStore) -> TestResult {
    let mut states = ReplayCollectStates::new();
    for _ in 0..64 {
        let node = workflow.node(frame.pc()).ok_or("replay node missing")?;
        let action = replay_step_with_collect(node, frame, store, workflow, &mut states)
            .map_err(|error| format!("replay step failed: {error:?}"))?;
        match action {
            ReplayAction::Continue(_) => {}
            ReplayAction::Finished => return Ok(()),
            _ => return Err("unexpected replay suspension".into()),
        }
    }
    Err("replay step budget exhausted".into())
}

fn node_kind<F, T>(workflow: &CompiledWorkflow, pick: F, missing: &str) -> TestResult<T>
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

fn accumulator(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    node_kind(
        workflow,
        |kind| match kind {
            CompiledNodeKind::ReduceStart { accumulator, .. } => Some(*accumulator),
            _ => None,
        },
        "ReduceStart absent",
    )
}

fn reduce_input_slot(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    (0..=u16::MAX)
        .find_map(|raw| {
            let node = workflow.node(StepIdx::new(raw))?;
            match node.kind {
                CompiledNodeKind::Copy { source } if source == SlotIdx::new(INPUT) => {
                    node.output.slot()
                }
                _ => None,
            }
        })
        .ok_or_else(|| "Reduce input Copy absent".into())
}

fn finish_result(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    node_kind(
        workflow,
        |kind| match kind {
            CompiledNodeKind::Finish { result } => Some(*result),
            _ => None,
        },
        "Finish absent",
    )
}

fn slots(frame: &RunFrame) -> TestResult<Vec<(SlotIdx, SlotValue, Taint)>> {
    let mut entries = frame.initialized_slots()?;
    entries.sort_by_key(|entry| entry.0.get());
    Ok(entries)
}

fn assert_source(
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

fn live_and_replay(values: &[i64], expected: i64) -> TestResult {
    let workflow = compile(SOURCE)?;
    let values: Vec<_> = values.iter().copied().map(SlotValue::I64).collect();
    let (mut frame, mut store, list) = bind(
        &workflow,
        SlotIdx::new(INPUT),
        &values,
        Taint::DerivedFromSecret,
        &[],
    )?;
    let signal = drive(
        &workflow,
        &mut frame,
        &mut store,
        128,
        &mut EvidenceCollector::new(),
    )?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(expected)));
    let result = finish_result(&workflow)?;
    assert_eq!(frame.read_slot(result), Ok(&SlotValue::I64(expected)));
    assert_eq!(frame.read_taint(result), Ok(Taint::Clean));
    assert_eq!(
        frame.read_slot(accumulator(&workflow)?),
        Ok(&SlotValue::I64(expected))
    );
    assert_source(&frame, &store, list, &values, Taint::DerivedFromSecret);

    let (mut replayed, mut replay_store, replay_list) = bind(
        &workflow,
        SlotIdx::new(INPUT),
        &values,
        Taint::DerivedFromSecret,
        &[],
    )?;
    replay(&workflow, &mut replayed, &mut replay_store)?;
    assert_eq!(
        (replayed.pc(), replayed.executed()),
        (frame.pc(), frame.executed())
    );
    assert_eq!(slots(&replayed)?, slots(&frame)?);
    assert_eq!(replayed.read_slot(result), Ok(&SlotValue::I64(expected)));
    assert_source(
        &replayed,
        &replay_store,
        replay_list,
        &values,
        Taint::DerivedFromSecret,
    );
    Ok(())
}

#[test]
fn source_reduce_live_and_replay_parity_for_empty_single_and_multiple_items() -> TestResult {
    live_and_replay(&[], 10)?;
    live_and_replay(&[4], 11)?;
    live_and_replay(&[4, 7, -3], 13)
}

#[test]
fn source_reduce_terminal_evidence_names_accumulator_input_and_iterator_writes() -> TestResult {
    let workflow = compile(SOURCE)?;
    let values = [SlotValue::I64(4), SlotValue::I64(7)];
    let (mut frame, mut store, _list) =
        bind(&workflow, SlotIdx::new(INPUT), &values, Taint::Secret, &[])?;
    let mut evidence = EvidenceCollector::new();
    drive(&workflow, &mut frame, &mut store, 128, &mut evidence)?;
    let events = evidence.drain();
    let accumulator = accumulator(&workflow)?;
    let reduce_input = reduce_input_slot(&workflow)?;
    assert!(events.contains(&EvidenceEvent::SlotWritten {
        slot: accumulator,
        value: SlotValue::I64(10),
        taint: Taint::Clean,
        extra: None,
    }));
    assert!(events.contains(&EvidenceEvent::SlotWritten {
        slot: reduce_input,
        value: values[0],
        taint: Taint::Secret,
        extra: None,
    }));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, EvidenceEvent::SlotWritten {
        slot, value: SlotValue::List(_), taint: Taint::Secret, extra: None
    } if *slot != reduce_input && *slot != SlotIdx::new(INPUT)))
    );
    assert!(events.contains(&EvidenceEvent::SlotWritten {
        slot: accumulator,
        value: SlotValue::I64(12),
        taint: Taint::Clean,
        extra: None,
    }));
    Ok(())
}

fn reopen(frame: &RunFrame, store: &ValueStore) -> TestResult<(RunFrame, ValueStore)> {
    let frame_bytes = postcard::to_allocvec(&frame.checkpoint_v1())?;
    let frame = RunFrame::from_checkpoint_v1(postcard::from_bytes(&frame_bytes)?)
        .map_err(|error| format!("frame restore failed: {error:?}"))?;
    let store_bytes = postcard::to_allocvec(&store.checkpoint_v1())?;
    let store = ValueStore::from_checkpoint_v1(postcard::from_bytes(&store_bytes)?)?;
    Ok((frame, store))
}

#[test]
fn source_reduce_cold_reopen_continuation_matches_uninterrupted_at_every_boundary() -> TestResult {
    let workflow = compile(SOURCE)?;
    let values = [SlotValue::I64(4), SlotValue::I64(7)];
    let (mut reference, mut ref_store, _) = bind(
        &workflow,
        SlotIdx::new(INPUT),
        &values,
        Taint::DerivedFromSecret,
        &[],
    )?;
    let mut full_evidence = EvidenceCollector::new();
    assert_eq!(
        drive(
            &workflow,
            &mut reference,
            &mut ref_store,
            128,
            &mut full_evidence
        )?,
        RuntimeSignal::Finished(SlotValue::I64(12))
    );
    let full = full_evidence.drain();
    let expected_slots = slots(&reference)?;
    let total_steps = reference.executed();
    for stop in 1..total_steps {
        let (mut frame, mut store, _) = bind(
            &workflow,
            SlotIdx::new(INPUT),
            &values,
            Taint::DerivedFromSecret,
            &[],
        )?;
        let mut prefix = EvidenceCollector::new();
        assert_eq!(
            drive(&workflow, &mut frame, &mut store, stop, &mut prefix)?,
            RuntimeSignal::StepBudgetExhausted
        );
        let (mut restored, mut restored_store) = reopen(&frame, &store)?;
        assert_eq!(restored.checkpoint_v1(), frame.checkpoint_v1());
        assert_eq!(restored_store.checkpoint_v1(), store.checkpoint_v1());
        let mut suffix = EvidenceCollector::new();
        assert_eq!(
            drive(
                &workflow,
                &mut restored,
                &mut restored_store,
                128,
                &mut suffix
            )?,
            RuntimeSignal::Finished(SlotValue::I64(12))
        );
        let mut joined = prefix.drain();
        joined.extend(suffix.drain());
        assert_eq!(
            joined, full,
            "evidence diverged after reopen at step {stop}"
        );
        assert_eq!(
            (restored.pc(), restored.executed()),
            (reference.pc(), reference.executed())
        );
        assert_eq!(slots(&restored)?, expected_slots);
    }
    Ok(())
}

#[test]
fn source_reduce_cold_reopen_replay_continuation_matches_live() -> TestResult {
    let workflow = compile(SOURCE)?;
    let values = [SlotValue::I64(4), SlotValue::I64(7)];
    let (mut reference, mut ref_store, _) = bind(
        &workflow,
        SlotIdx::new(INPUT),
        &values,
        Taint::DerivedFromSecret,
        &[],
    )?;
    drive(
        &workflow,
        &mut reference,
        &mut ref_store,
        128,
        &mut EvidenceCollector::new(),
    )?;
    for stop in [2, 4] {
        let (mut frame, mut store, _) = bind(
            &workflow,
            SlotIdx::new(INPUT),
            &values,
            Taint::DerivedFromSecret,
            &[],
        )?;
        drive(
            &workflow,
            &mut frame,
            &mut store,
            stop,
            &mut EvidenceCollector::new(),
        )?;
        let (mut restored, mut restored_store) = reopen(&frame, &store)?;
        replay(&workflow, &mut restored, &mut restored_store)?;
        assert_eq!(
            (restored.pc(), restored.executed()),
            (reference.pc(), reference.executed())
        );
        assert_eq!(slots(&restored)?, slots(&reference)?);
        assert_source(
            &restored,
            &restored_store,
            match restored.read_slot(SlotIdx::new(INPUT))? {
                SlotValue::List(list) => *list,
                other => return Err(format!("input root not a list: {other:?}").into()),
            },
            &values,
            Taint::DerivedFromSecret,
        );
    }
    Ok(())
}

#[test]
fn source_reduce_two_sequential_activations_match_live_and_replay() -> TestResult {
    let workflow = compile(TWO)?;
    let (mut frame, mut store, _) = bind(
        &workflow,
        SlotIdx::new(INPUT),
        &[SlotValue::I64(4), SlotValue::I64(7), SlotValue::I64(-3)],
        Taint::Clean,
        &[],
    )?;
    let second = SlotIdx::new(9);
    let second_list = store.insert_list(Box::from([SlotValue::I64(1), SlotValue::I64(2)]))?;
    frame.write_slot(second, SlotValue::List(second_list))?;
    let signal = drive(
        &workflow,
        &mut frame,
        &mut store,
        256,
        &mut EvidenceCollector::new(),
    )?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(2)));
    let mut accumulators = (0..=u16::MAX).filter_map(|raw| {
        workflow
            .node(StepIdx::new(raw))
            .and_then(|node| match node.kind {
                CompiledNodeKind::ReduceStart { accumulator, .. } => Some(accumulator),
                _ => None,
            })
    });
    let (first_acc, second_acc) = (
        accumulators.next().ok_or("first accumulator")?,
        accumulators.next().ok_or("second accumulator")?,
    );
    assert_eq!(frame.read_slot(first_acc), Ok(&SlotValue::I64(13)));
    assert_eq!(frame.read_slot(second_acc), Ok(&SlotValue::I64(2)));
    let (mut replayed, mut replay_store, _) = bind(
        &workflow,
        SlotIdx::new(INPUT),
        &[SlotValue::I64(4), SlotValue::I64(7), SlotValue::I64(-3)],
        Taint::Clean,
        &[],
    )?;
    let list = replay_store.insert_list(Box::from([SlotValue::I64(1), SlotValue::I64(2)]))?;
    replayed.write_slot(second, SlotValue::List(list))?;
    replay(&workflow, &mut replayed, &mut replay_store)?;
    assert_eq!(
        (replayed.pc(), replayed.executed()),
        (frame.pc(), frame.executed())
    );
    assert_eq!(slots(&replayed)?, slots(&frame)?);
    Ok(())
}

#[test]
fn source_reduce_preserves_source_taints_across_all_variants() -> TestResult {
    let workflow = compile(SOURCE)?;
    for taint in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ] {
        let values = [SlotValue::I64(3), SlotValue::I64(7)];
        let item_taints = [taint, Taint::Clean];
        let (mut frame, mut store, list) =
            bind(&workflow, SlotIdx::new(INPUT), &values, taint, &item_taints)?;
        drive(
            &workflow,
            &mut frame,
            &mut store,
            2,
            &mut EvidenceCollector::new(),
        )?;
        let reduce_input = reduce_input_slot(&workflow)?;
        assert_eq!(frame.read_slot(reduce_input), Ok(&values[0]));
        assert_eq!(frame.read_taint(reduce_input), Ok(taint));
        assert_source(&frame, &store, list, &values, taint);
        let (mut replayed, mut replay_store, replay_list) =
            bind(&workflow, SlotIdx::new(INPUT), &values, taint, &item_taints)?;
        for _ in 0..2 {
            let node = workflow.node(replayed.pc()).ok_or("replay node")?;
            replay_step_with_collect(
                node,
                &mut replayed,
                &mut replay_store,
                &workflow,
                &mut ReplayCollectStates::new(),
            )
            .map_err(|error| format!("replay step failed: {error:?}"))?;
        }
        assert_eq!(slots(&replayed)?, slots(&frame)?);
        assert_source(&replayed, &replay_store, replay_list, &values, taint);
    }
    Ok(())
}

fn parts(limit: u32, start_output: Option<SlotIdx>) -> WorkflowParts {
    WorkflowParts {
        name: "shipping_reduce_manual".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: Box::from([
            vb_core::workflow::node(
                StepIdx::ZERO,
                CompiledNodeKind::Copy {
                    source: SlotIdx::ZERO,
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(1)),
            ),
            vb_core::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::ReduceStart {
                    input: SlotIdx::new(1),
                    accumulator: SlotIdx::new(2),
                    initial: ConstIdx::new(0),
                    limit,
                    body: StepIdx::new(2),
                    done: StepIdx::new(4),
                },
                start_output,
                None,
            ),
            vb_core::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(3)),
            ),
            vb_core::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::ReduceNext {
                    iterator_slot: SlotIdx::new(3),
                    accumulator: SlotIdx::new(2),
                    body: StepIdx::new(2),
                    done: StepIdx::new(4),
                },
                Some(SlotIdx::new(5)),
                None,
            ),
            vb_core::workflow::node(
                StepIdx::new(4),
                CompiledNodeKind::ReduceFinish {
                    accumulator: SlotIdx::new(2),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(5)),
            ),
            vb_core::workflow::node(
                StepIdx::new(5),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::from([ConstValue::I64(10), ConstValue::I64(1)]),
        slot_count: 6,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::new([]),
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        input_slots: Box::from([vb_core::workflow::CompiledInputSlot {
            slot: SlotIdx::ZERO,
            kind: vb_core::workflow::InputSlotKind::List,
            constraint: vb_core::workflow::CompiledInputConstraint::required(),
        }]),
    }
}

fn manual(limit: u32, start_output: Option<SlotIdx>) -> TestResult<CompiledWorkflow> {
    let parts = parts(limit, start_output);
    vb_validate::shared::validate(&parts)?;
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn manual_frame(
    workflow: &CompiledWorkflow,
    items: &[SlotValue],
) -> TestResult<(RunFrame, ValueStore)> {
    let mut frame = RunFrame::new(
        RunId::new(7),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )?;
    frame.set_pc(StepIdx::new(1))?;
    let mut store = ValueStore::new();
    let list = store.insert_list(items.to_vec().into_boxed_slice())?;
    frame.write_slot(SlotIdx::new(1), SlotValue::List(list))?;
    Ok((frame, store))
}

#[test]
fn reduce_start_limit_overflow_is_typed_in_live_and_replay() -> TestResult {
    let workflow = manual(1, Some(SlotIdx::new(3)))?;
    let (mut frame, mut store) = manual_frame(&workflow, &[SlotValue::I64(1), SlotValue::I64(2)])?;
    let before = (frame.checkpoint_v1(), store.checkpoint_v1());
    let live = reduce_start(
        &workflow,
        &mut frame,
        &mut store,
        SlotIdx::new(1),
        SlotIdx::new(2),
        ConstIdx::new(0),
        1,
        StepIdx::new(2),
        StepIdx::new(4),
        Some(SlotIdx::new(3)),
    );
    assert_eq!(
        live,
        Err(vb_core::EngineError::IterationLimitExceeded {
            resource: "reduce_limit"
        })
    );
    assert_eq!((frame.checkpoint_v1(), store.checkpoint_v1()), before);
    let (mut frame, mut store) = manual_frame(&workflow, &[SlotValue::I64(1), SlotValue::I64(2)])?;
    let node = workflow
        .node(StepIdx::new(1))
        .ok_or("ReduceStart missing")?;
    let replay = replay_step_with_collect(
        node,
        &mut frame,
        &mut store,
        &workflow,
        &mut ReplayCollectStates::new(),
    );
    assert_eq!(
        replay.err(),
        Some(ReplayError::Internal {
            reason: "reduce item count exceeds limit during replay"
        })
    );
    assert_eq!((frame.checkpoint_v1(), store.checkpoint_v1()), before);
    Ok(())
}

#[test]
fn reduce_start_counter_overflow_is_typed_in_live_and_replay() -> TestResult {
    let workflow = manual(4, Some(SlotIdx::new(3)))?;
    let (frame, store) = manual_frame(&workflow, &[SlotValue::I64(1)])?;
    let mut checkpoint = frame.checkpoint_v1();
    checkpoint.executed = u64::MAX;
    let before_store = store.checkpoint_v1();
    let mut frame = RunFrame::from_checkpoint_v1(checkpoint.clone())
        .map_err(|error| format!("frame restore failed: {error:?}"))?;
    let mut store = store;
    let live = reduce_start(
        &workflow,
        &mut frame,
        &mut store,
        SlotIdx::new(1),
        SlotIdx::new(2),
        ConstIdx::new(0),
        4,
        StepIdx::new(2),
        StepIdx::new(4),
        Some(SlotIdx::new(3)),
    );
    assert_eq!(live, Err(vb_core::EngineError::StepCounterOverflow));
    assert_eq!(frame.checkpoint_v1(), checkpoint);
    assert_eq!(store.checkpoint_v1(), before_store);
    let mut frame = RunFrame::from_checkpoint_v1(checkpoint.clone())
        .map_err(|error| format!("frame restore failed: {error:?}"))?;
    let node = workflow
        .node(StepIdx::new(1))
        .ok_or("ReduceStart missing")?;
    let replay = replay_step_with_collect(
        node,
        &mut frame,
        &mut store,
        &workflow,
        &mut ReplayCollectStates::new(),
    );
    assert_eq!(
        replay.err(),
        Some(ReplayError::Internal {
            reason: "executed counter overflow"
        })
    );
    assert_eq!(frame.checkpoint_v1(), checkpoint);
    assert_eq!(store.checkpoint_v1(), before_store);
    Ok(())
}

fn state_frame(
    workflow: &CompiledWorkflow,
    state: &[SlotValue],
) -> TestResult<(RunFrame, ValueStore)> {
    let mut frame = manual_frame(workflow, &[])?;
    frame.0.set_pc(StepIdx::new(3))?;
    frame.0.write_slot(SlotIdx::new(2), SlotValue::I64(10))?;
    let list = frame.1.insert_list(state.to_vec().into_boxed_slice())?;
    frame.0.write_slot(SlotIdx::new(3), SlotValue::List(list))?;
    Ok(frame)
}

fn assert_iterator_handle_refusal(
    state: &[SlotValue],
    live_error: vb_core::EngineError,
    replay_error: ReplayError,
) -> TestResult {
    let workflow = manual(4, Some(SlotIdx::new(3)))?;
    let (mut frame, mut store) = state_frame(&workflow, state)?;
    let before = (frame.checkpoint_v1(), store.checkpoint_v1());
    let live = reduce_next(
        &mut frame,
        &mut store,
        SlotIdx::new(3),
        SlotIdx::new(2),
        StepIdx::new(2),
        StepIdx::new(4),
        Some(SlotIdx::new(5)),
    );
    assert_eq!(live, Err(live_error));
    assert_eq!((frame.checkpoint_v1(), store.checkpoint_v1()), before);
    let mut frame = RunFrame::from_checkpoint_v1(before.0.clone())
        .map_err(|error| format!("frame restore failed: {error:?}"))?;
    let mut store = ValueStore::from_checkpoint_v1(before.1.clone())?;
    let node = workflow.node(StepIdx::new(3)).ok_or("ReduceNext missing")?;
    let replay = replay_step_with_collect(
        node,
        &mut frame,
        &mut store,
        &workflow,
        &mut ReplayCollectStates::new(),
    );
    assert_eq!(replay.err(), Some(replay_error));
    assert_eq!((frame.checkpoint_v1(), store.checkpoint_v1()), before);
    Ok(())
}

#[test]
fn reduce_next_rejects_empty_iterator_handle_in_live_and_replay() -> TestResult {
    assert_iterator_handle_refusal(
        &[],
        vb_core::EngineError::InternalInvariantViolation {
            reason: "iterator state must be a 2-element list".into(),
        },
        ReplayError::Internal {
            reason: "iterator state must be a 2-element list during replay",
        },
    )
}

#[test]
fn reduce_next_rejects_non_two_element_iterator_handle_in_live_and_replay() -> TestResult {
    assert_iterator_handle_refusal(
        &[SlotValue::I64(0), SlotValue::I64(0), SlotValue::I64(0)],
        vb_core::EngineError::InternalInvariantViolation {
            reason: "iterator state must be a 2-element list".into(),
        },
        ReplayError::Internal {
            reason: "iterator state must be a 2-element list during replay",
        },
    )
}

#[test]
fn reduce_next_rejects_out_of_range_iterator_source_token_in_live_and_replay() -> TestResult {
    assert_iterator_handle_refusal(
        &[SlotValue::I64(-1), SlotValue::I64(0)],
        vb_core::EngineError::InternalInvariantViolation {
            reason: "iterator state source token out of u32 range".into(),
        },
        ReplayError::Internal {
            reason: "iterator source token out of u32 range during replay",
        },
    )
}

#[path = "shipping_reduce_recovery/journal_recovery.rs"]
mod journal_recovery;
