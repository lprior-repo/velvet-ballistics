use crate::{
    CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ResourceContract, RunFrame, RunId,
    SlotIdx, SlotValue, StepIdx, Taint, ValueStore, WorkflowDigest, WorkflowParts,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn constant_plan() -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: "replay_compound_constants".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: Box::from([crate::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Nop,
            None,
            None,
        )]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::from([ConstValue::I64(17)]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::new([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::new([]),
    })
}

#[test]
fn together_join_keeps_null_list_and_item_taints() -> TestResult {
    for kind in 0..3 {
        let mut store = ValueStore::new();
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4)?;
        run.set_max_parallel_in_flight(2);
        run.add_parallel_in_flight(2)?;
        let original = store
            .insert_list_with_taint(Box::from([SlotValue::I64(11)]), Box::from([Taint::Secret]))?;
        let last = match kind {
            0 => SlotValue::Null,
            1 => SlotValue::List(store.insert_list(Box::from([SlotValue::I64(23)]))?),
            _ => SlotValue::I64(29),
        };
        run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::List(original), Taint::Secret)?;
        run.write_slot_with_taint(SlotIdx::new(1), last, Taint::TimeDependent)?;
        let node = crate::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::TogetherJoin {
                branch_count: 2,
                accumulator: SlotIdx::ZERO,
            },
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(1)),
        );
        let action =
            super::together::replay_together_join(&mut run, &mut store, &node, 2, SlotIdx::ZERO)
                .map_err(|error| format!("replay join: {error:?}"))?;
        assert!(matches!(action, super::ReplayAction::Continue(step) if step == StepIdx::new(1)));
        let SlotValue::List(result) = *run.read_slot(SlotIdx::new(1))? else {
            return Err("Together result must be a list".into());
        };
        assert_eq!(store.list(result)?, &[SlotValue::I64(11), last]);
        assert_eq!(store.list_item_with_taint(result, 0)?.1, Taint::Secret);
        assert_eq!(
            store.list_item_with_taint(result, 1)?.1,
            Taint::TimeDependent
        );
        assert_eq!(store.list(original)?, &[SlotValue::I64(11)]);
        assert_eq!(run.parallel_in_flight(), 0);
        assert_eq!(run.executed(), 1);
    }
    Ok(())
}

#[test]
fn compound_starts_preserve_frame_and_store_at_counter_exhaustion() -> TestResult {
    let plan = constant_plan()?;
    for family in 0..3 {
        let mut store = ValueStore::new();
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4)?;
        run.set_max_parallel_in_flight(2);
        let source = store.insert_list(Box::from([SlotValue::I64(7)]))?;
        run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::List(source), Taint::Secret)?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(42), Taint::TimeDependent)?;
        let mut checkpoint = run.checkpoint_v1();
        checkpoint.executed = u64::MAX;
        let mut run = RunFrame::from_checkpoint_v1(checkpoint.clone())
            .map_err(|error| format!("frame restore: {error:?}"))?;
        let before_store = store.checkpoint_v1();
        let node = crate::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Nop,
            Some(SlotIdx::new(2)),
            Some(StepIdx::new(1)),
        );
        let result = match family {
            0 => super::for_each::replay_for_each_start(
                &mut run,
                &mut store,
                &node,
                SlotIdx::ZERO,
                SlotIdx::new(1),
                1,
                StepIdx::new(1),
                StepIdx::new(1),
            ),
            1 => super::reduce::replay_reduce_start(
                &plan,
                &mut run,
                &mut store,
                &node,
                SlotIdx::ZERO,
                SlotIdx::new(1),
                ConstIdx::new(0),
                1,
                StepIdx::new(1),
                StepIdx::new(1),
            ),
            _ => super::together::replay_together_start(
                &mut run,
                &mut store,
                &node,
                &[StepIdx::new(1)],
            ),
        };
        assert!(result.is_err());
        assert_eq!(run.checkpoint_v1(), checkpoint, "family {family}");
        assert_eq!(store.checkpoint_v1(), before_store, "family {family}");
    }
    Ok(())
}

#[test]
fn reduce_rejects_oversized_source_before_accumulator_write() -> TestResult {
    let plan = constant_plan()?;
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4)?;
    let source = store.insert_list(Box::from([SlotValue::I64(7)]))?;
    run.write_slot(SlotIdx::ZERO, SlotValue::List(source))?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(42), Taint::Secret)?;
    let before = run.checkpoint_v1();
    let before_store = store.checkpoint_v1();
    let node = crate::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Nop,
        Some(SlotIdx::new(2)),
        Some(StepIdx::new(1)),
    );
    let result = super::reduce::replay_reduce_start(
        &plan,
        &mut run,
        &mut store,
        &node,
        SlotIdx::ZERO,
        SlotIdx::new(1),
        ConstIdx::new(0),
        0,
        StepIdx::new(1),
        StepIdx::new(1),
    );
    assert!(matches!(
        result,
        Err(crate::replay::ReplayError::Internal {
            reason: "reduce item count exceeds limit during replay"
        })
    ));
    assert_eq!(run.checkpoint_v1(), before);
    assert_eq!(store.checkpoint_v1(), before_store);
    Ok(())
}
