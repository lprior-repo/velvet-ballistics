use vb_core::{
    CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ResourceContract, RunFrame, RunId,
    SlotIdx, SlotValue, StepIdx, Taint, ValueStore, WorkflowDigest, WorkflowParts,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn constants() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    Ok(CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: "iteration_taints".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Nop,
            None,
            None,
        )]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(0)]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    })?)
}
fn iteration_contract(reduce: bool) -> TestResult {
    let plan = constants()?;
    let variants = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for selected in variants {
        for replay in [false, true] {
            for root in [
                Taint::Clean,
                Taint::DerivedFromSecret,
                Taint::Secret,
                Taint::Random,
                Taint::TimeDependent,
            ] {
                let mut store = ValueStore::new();
                let source = store.insert_list_with_taint(
                    Box::from([SlotValue::I64(7), SlotValue::I64(9)]),
                    Box::from([selected, Taint::Clean]),
                )?;
                let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 4)?;
                run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::List(source), root)?;
                let kind = if reduce {
                    CompiledNodeKind::ReduceStart {
                        input: SlotIdx::ZERO,
                        accumulator: SlotIdx::new(1),
                        initial: ConstIdx::new(0),
                        limit: 2,
                        body: StepIdx::new(1),
                        done: StepIdx::new(3),
                    }
                } else {
                    CompiledNodeKind::ForEachStart {
                        input: SlotIdx::ZERO,
                        item_slot: SlotIdx::new(1),
                        limit: 2,
                        body: StepIdx::new(1),
                        done: StepIdx::new(3),
                    }
                };
                let start =
                    vb_core::workflow::node(StepIdx::ZERO, kind, Some(SlotIdx::new(2)), None);
                if replay {
                    vb_core::replay::step::replay_step(&start, &mut run, &mut store, &plan)
                        .map_err(|e| format!("{e:?}"))?;
                } else if reduce {
                    super::reduce::reduce_start(
                        &plan,
                        &mut run,
                        &mut store,
                        SlotIdx::ZERO,
                        SlotIdx::new(1),
                        ConstIdx::new(0),
                        2,
                        StepIdx::new(1),
                        StepIdx::new(3),
                        Some(SlotIdx::new(2)),
                    )?;
                } else {
                    super::for_each::for_each_start(
                        &mut run,
                        &mut store,
                        SlotIdx::ZERO,
                        SlotIdx::new(1),
                        2u32,
                        StepIdx::new(1),
                        StepIdx::new(3),
                        Some(SlotIdx::new(2)),
                    )?;
                }
                let item = if reduce {
                    SlotIdx::ZERO
                } else {
                    SlotIdx::new(1)
                };
                assert_eq!(*run.read_slot(item)?, SlotValue::I64(7));
                assert_eq!(
                    run.read_taint(item)?,
                    vb_core::value::join_taint(root, selected)
                );
                assert_eq!(run.read_taint(SlotIdx::new(2))?, root);
                let next_kind = if reduce {
                    CompiledNodeKind::ReduceNext {
                        iterator_slot: SlotIdx::new(2),
                        accumulator: SlotIdx::new(1),
                        body: StepIdx::new(1),
                        done: StepIdx::new(3),
                    }
                } else {
                    CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(2),
                        body: StepIdx::new(1),
                        done: StepIdx::new(3),
                    }
                };
                let next = vb_core::workflow::node(StepIdx::new(2), next_kind, Some(item), None);
                if replay {
                    vb_core::replay::step::replay_step(&next, &mut run, &mut store, &plan)
                        .map_err(|e| format!("{e:?}"))?;
                } else if reduce {
                    super::reduce::reduce_next(
                        &mut run,
                        &mut store,
                        SlotIdx::new(2),
                        SlotIdx::new(1),
                        StepIdx::new(1),
                        StepIdx::new(3),
                        Some(item),
                    )?;
                } else {
                    super::for_each::for_each_next(
                        &mut run,
                        &mut store,
                        SlotIdx::new(2),
                        StepIdx::new(1),
                        StepIdx::new(3),
                        Some(item),
                    )?;
                }
                assert_eq!(*run.read_slot(item)?, SlotValue::I64(9));
                assert_eq!(run.read_taint(item)?, root);
                assert_eq!(run.read_taint(SlotIdx::new(2))?, root);
                assert_eq!(
                    store.list_item_with_taint(source, 0)?,
                    (SlotValue::I64(7), selected)
                );
                assert_eq!(
                    store.list_item_with_taint(source, 1)?,
                    (SlotValue::I64(9), Taint::Clean)
                );
            }
        }
    }
    Ok(())
}
#[test]
fn for_each_selected_item_preserves_taint_without_cursor_contamination() -> TestResult {
    iteration_contract(false)
}
#[test]
fn reduce_selected_item_preserves_taint_without_cursor_contamination() -> TestResult {
    iteration_contract(true)
}

fn collect_contract(replay: bool) -> TestResult {
    let variants = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for root_taint in variants {
        for selected_taint in variants {
            let plan = constants()?;
            let mut store = ValueStore::new();
            let source = store.insert_list_with_taint(
                Box::from([SlotValue::I64(7), SlotValue::I64(9)]),
                Box::from([selected_taint, Taint::Clean]),
            )?;
            let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 4)?;
            run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::List(source), root_taint)?;
            let start = vb_core::workflow::node(
                StepIdx::ZERO,
                CompiledNodeKind::CollectStart {
                    source: SlotIdx::ZERO,
                    limit: 2,
                    page_size: 1,
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(2)),
                None,
            );
            let next = vb_core::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::CollectNext {
                    collector_slot: SlotIdx::new(2),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                None,
                None,
            );
            let mut live_states = super::collect::CollectStates::new();
            let mut replay_states = vb_core::replay::step::ReplayCollectStates::new();
            if replay {
                vb_core::replay::step::replay_step_with_collect(
                    &start,
                    &mut run,
                    &mut store,
                    &plan,
                    &mut replay_states,
                )
                .map_err(|e| format!("{e:?}"))?;
            } else {
                super::collect::collect_start(
                    &mut run,
                    &mut store,
                    &mut live_states,
                    SlotIdx::ZERO,
                    2,
                    1,
                    StepIdx::new(1),
                    StepIdx::new(3),
                    Some(SlotIdx::new(2)),
                    None,
                )?;
            }
            for (item, expected_taint) in [(7, selected_taint), (9, Taint::Clean)] {
                let SlotValue::List(page) = *run.read_slot(SlotIdx::new(2))? else {
                    return Err("page".into());
                };
                assert_eq!(
                    store.list_item_with_taint(page, 0)?,
                    (SlotValue::I64(item), expected_taint)
                );
                assert_eq!(run.read_taint(SlotIdx::new(2))?, root_taint);
                if item == 7 {
                    if replay {
                        vb_core::replay::step::replay_step_with_collect(
                            &next,
                            &mut run,
                            &mut store,
                            &plan,
                            &mut replay_states,
                        )
                        .map_err(|e| format!("{e:?}"))?;
                    } else {
                        super::collect::collect_next(
                            &mut run,
                            &mut store,
                            &mut live_states,
                            SlotIdx::new(2),
                            StepIdx::new(1),
                            StepIdx::new(3),
                        )?;
                    }
                }
            }
            assert_eq!(
                store.list_item_with_taint(source, 0)?,
                (SlotValue::I64(7), selected_taint)
            );
        }
    }
    Ok(())
}
#[test]
fn live_collect_page_preserves_item_taints() -> TestResult {
    collect_contract(false)
}
#[test]
fn replay_collect_page_preserves_item_taints() -> TestResult {
    collect_contract(true)
}
