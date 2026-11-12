// ---- BuildObject multiple fields ----

#[test]
fn replay_build_object_multiple_fields_preserves_order() -> Result<(), CoreError> {
    let sym_a = SymbolId::new(0);
    let sym_b = SymbolId::new(1);
    let plan = make_plan_with_symbols(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::BuildObject {
                    fields: vec![(sym_a, SlotIdx::new(0)), (sym_b, SlotIdx::new(1))].into(),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(10), ConstValue::I64(20)],
        vec![],
        2,
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing".into(),
        })?;
    replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(2))? {
        SlotValue::Object(id) => {
            let fields = store.object(id)?;
            if fields.len() != 2 {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "object should have 2 fields".into(),
                });
            }
            if fields[0].key != sym_a
                || fields[0].value != SlotValue::I64(10)
                || fields[1].key != sym_b
                || fields[1].value != SlotValue::I64(20)
            {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "field order or values wrong".into(),
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 2 should be Object".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_collect_next_reports_missing_source_list() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::CollectNext {
                    collector_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                None,
                None,
            ),
            crate::workflow::node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
            crate::workflow::node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        ],
        vec![],
        vec![],
    )?;
    let mut store = ValueStore::new();
    let current_page = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "current page insert failed".into(),
        })?;
    let mut run = RunFrame::new(
        RunId::new(91),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(current_page))?;
    let mut states = super::ReplayCollectStates::new();
    states.upsert(
        SlotIdx::new(0),
        super::ReplayCollectState {
            source: ListId::new(99),
            current_page,
            cursor: 1,
            page_size: 1,
            item_count: 1,
            taint: Taint::Clean,
        },
    );
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    match super::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states) {
        Err(ReplayError::Internal { reason })
            if reason == "collect source list missing during replay" =>
        {
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "CollectNext should report missing source list".into(),
        }),
    }
}

#[test]
fn replay_copy_reports_source_slot_failures() -> Result<(), CoreError> {
    let node = crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Copy {
            source: SlotIdx::new(7),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    );
    let mut run = RunFrame::new(RunId::new(92), StepIdx::new(0), 2, 1)?;
    match super::replay_copy(&mut run, &node, SlotIdx::new(7)) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(7) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Copy should report out-of-bounds source slot".into(),
            });
        }
    }
    match super::replay_copy(&mut run, &node, SlotIdx::new(0)) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "Copy should report uninitialized source slot".into(),
        }),
    }
}

#[test]
fn replay_build_object_reports_field_slot_failures() -> Result<(), CoreError> {
    let node = crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::BuildObject {
            fields: vec![].into(),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    );
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(93), StepIdx::new(0), 2, 1)?;
    let fields = [(SymbolId::new(0), SlotIdx::new(7))];
    match super::replay_build_object(&mut run, &mut store, &node, &fields) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(7) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BuildObject should report out-of-bounds field slot".into(),
            });
        }
    }
    let fields = [(SymbolId::new(0), SlotIdx::new(0))];
    match super::replay_build_object(&mut run, &mut store, &node, &fields) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "BuildObject should report uninitialized field slot".into(),
        }),
    }
}

#[test]
fn replay_build_list_reports_item_slot_failures() -> Result<(), CoreError> {
    let node = crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::BuildList {
            items: vec![].into(),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    );
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(94), StepIdx::new(0), 2, 1)?;
    match super::replay_build_list(&mut run, &mut store, &node, &[SlotIdx::new(7)]) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(7) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BuildList should report out-of-bounds item slot".into(),
            });
        }
    }
    match super::replay_build_list(&mut run, &mut store, &node, &[SlotIdx::new(0)]) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "BuildList should report uninitialized item slot".into(),
        }),
    }
}

#[test]
fn replay_finish_reports_result_slot_failures() -> Result<(), CoreError> {
    let mut run = RunFrame::new(RunId::new(95), StepIdx::new(0), 1, 1)?;
    match super::replay_finish(&mut run, SlotIdx::new(7)) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(7) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Finish should report out-of-bounds result slot".into(),
            });
        }
    }
    match super::replay_finish(&mut run, SlotIdx::new(0)) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "Finish should report uninitialized result slot".into(),
        }),
    }
}
