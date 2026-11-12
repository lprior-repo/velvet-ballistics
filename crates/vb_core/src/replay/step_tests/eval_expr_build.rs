// ---- EvalExpr step ----

#[test]
fn replay_eval_expr_computes_result() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ])?;

    let plan = make_plan(
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
                CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
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
        vec![ConstValue::I64(30), ConstValue::I64(12)],
        vec![expr],
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

    if *run.read_slot(SlotIdx::new(2))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 2 should be I64(42)".into(),
        });
    }
    Ok(())
}

// ---- BuildObject step ----

#[test]
fn replay_build_object_creates_handle() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let plan = make_plan(
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
                CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(0))].into(),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(42)],
        vec![],
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

    match *run.read_slot(SlotIdx::new(1))? {
        SlotValue::Object(id) => {
            let obj = store.object(id)?;
            let field = obj.first().ok_or(CoreError::InternalInvariantViolation {
                reason: "object should have a field".into(),
            })?;
            if field.key != field_sym || field.value != SlotValue::I64(42) {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "field mismatch".into(),
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 1 should be Object".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_object_empty_fields() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::BuildObject {
                    fields: vec![].into(),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing".into(),
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(0))? {
        SlotValue::Object(id) => {
            let obj = store.object(id)?;
            if !obj.is_empty() {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "empty BuildObject should create empty object".into(),
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 0 should be Object".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_object_uninitialized_field_returns_error() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(5))].into(),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing".into(),
        })?;
    let result = replay_step(node, &mut run, &mut store, &plan);
    assert!(
        result.is_err(),
        "BuildObject with uninitialized field must fail"
    );
    Ok(())
}

#[test]
fn replay_build_object_propagates_taint() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let plan = make_plan(
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
                CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(0))].into(),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(1)],
        vec![],
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
    run.write_taint(SlotIdx::new(0), Taint::DerivedFromSecret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.read_taint(SlotIdx::new(1))? != Taint::DerivedFromSecret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BuildObject output taint should be DerivedFromSecret".into(),
        });
    }
    Ok(())
}

// ---- BuildList step ----

#[test]
fn replay_build_list_creates_handle() -> Result<(), CoreError> {
    let plan = make_plan(
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
                CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0), SlotIdx::new(1)].into(),
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
        SlotValue::List(id) => {
            let list = store.list(id)?;
            if list.len() != 2 {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "list should have 2 items".into(),
                });
            }
            if list[0] != SlotValue::I64(10) || list[1] != SlotValue::I64(20) {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "list items mismatch".into(),
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 2 should be List".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_list_empty_items() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::BuildList {
                    items: vec![].into(),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing".into(),
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(0))? {
        SlotValue::List(id) => {
            let list = store.list(id)?;
            if !list.is_empty() {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "empty BuildList should create empty list".into(),
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 0 should be List".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_list_uninitialized_item_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(5)].into(),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing".into(),
        })?;
    let result = replay_step(node, &mut run, &mut store, &plan);
    assert!(
        result.is_err(),
        "BuildList with uninitialized item must fail"
    );
    Ok(())
}

#[test]
fn replay_build_list_propagates_taint() -> Result<(), CoreError> {
    let plan = make_plan(
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
                CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0)].into(),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(7)],
        vec![],
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
    run.write_taint(SlotIdx::new(0), Taint::Secret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.read_taint(SlotIdx::new(1))? != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BuildList output taint should be Secret".into(),
        });
    }
    Ok(())
}
