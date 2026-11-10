// =========================================================================
// Replay engine: idempotency, equivalence, independent runs
// =========================================================================

#[test]
fn replay_step_by_step_equals_full_replay() -> Result<(), CoreError> {
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
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(1),
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
        vec![ConstValue::I64(100), ConstValue::I64(200)],
        vec![],
    )?;

    let mut full_store = ValueStore::new();
    let full_frame = ReplayEngine::new(&plan)
        .replay_frame_up_to(StepIdx::new(3), &mut full_store)
        .map_err(replay_err_to_core)?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut step_store = ValueStore::new();
    let mut step_run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let nodes: [u16; 3] = [0, 1, 2];
    nodes.iter().try_for_each(|&idx| {
        let node = plan
            .node(StepIdx::new(idx))
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "node missing",
            })?;
        super::step::replay_step(node, &mut step_run, &mut step_store, &plan)
            .map(|_| ())
            .map_err(replay_err_to_core)
    })?;

    assert_eq!(step_run.pc(), full_frame.pc());
    for slot in 0..3u16 {
        let step_val = *step_run.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "step frame read error",
            }
        })?;
        let full_val = *full_frame.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "full frame read error",
            }
        })?;
        assert_eq!(step_val, full_val, "slot {slot} mismatch");
    }
    Ok(())
}

#[test]
fn replay_idempotent_twice_returns_same_result() -> Result<(), CoreError> {
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
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(10), ConstValue::I64(20)],
        vec![],
    )?;

    let mut store_a = ValueStore::new();
    let frame_a = ReplayEngine::new(&plan)
        .replay_frame_up_to(StepIdx::new(2), &mut store_a)
        .map_err(replay_err_to_core)?;

    let mut store_b = ValueStore::new();
    let frame_b = ReplayEngine::new(&plan)
        .replay_frame_up_to(StepIdx::new(2), &mut store_b)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame_a.pc(), frame_b.pc());
    for slot in 0..2u16 {
        let val_a = *frame_a.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "frame_a read error",
            }
        })?;
        let val_b = *frame_b.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "frame_b read error",
            }
        })?;
        assert_eq!(val_a, val_b, "slot {slot} mismatch between idempotent runs");
    }
    Ok(())
}

#[test]
fn independent_runs_with_same_plan_produce_identical_state() -> Result<(), CoreError> {
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
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
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
        vec![ConstValue::I64(77), ConstValue::I64(88)],
        vec![],
    )?;

    let engine_a = ReplayEngine::new(&plan);
    let engine_b = ReplayEngine::new(&plan);
    let mut store_a = ValueStore::new();
    let mut store_b = ValueStore::new();

    let frame_a = engine_a
        .replay_frame_up_to(StepIdx::new(3), &mut store_a)
        .map_err(replay_err_to_core)?;
    let frame_b = engine_b
        .replay_frame_up_to(StepIdx::new(3), &mut store_b)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame_a.pc(), frame_b.pc());
    for slot in 0..3u16 {
        let val_a = *frame_a.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "frame_a read error",
            }
        })?;
        let val_b = *frame_b.read_slot(SlotIdx::new(slot)).map_err(|_| {
            CoreError::InternalInvariantViolation {
                reason: "frame_b read error",
            }
        })?;
        assert_eq!(val_a, val_b, "independent runs slot {slot} mismatch");
    }
    Ok(())
}

#[test]
fn replay_from_intermediate_snapshot() -> Result<(), CoreError> {
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
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(2),
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
        vec![
            ConstValue::I64(10),
            ConstValue::I64(20),
            ConstValue::I64(30),
        ],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame_full = ReplayEngine::new(&plan)
        .replay_frame_up_to(StepIdx::new(3), &mut store)
        .map_err(replay_err_to_core)?;

    let mut store_snap = ValueStore::new();
    let snap_frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(1), &mut store_snap)
        .map_err(replay_err_to_core)?;

    assert_eq!(*snap_frame.read_slot(SlotIdx::new(0))?, SlotValue::I64(10));
    assert_eq!(*snap_frame.read_slot(SlotIdx::new(1))?, SlotValue::I64(20));
    assert_eq!(snap_frame.pc(), StepIdx::new(2));

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut resume_store = ValueStore::new();
    let mut resume_run = RunFrame::new(RunId::new(0), snap_frame.pc(), step_count, slot_count)?;
    resume_run.write_slot(SlotIdx::new(0), SlotValue::I64(10))?;
    resume_run.write_slot(SlotIdx::new(1), SlotValue::I64(20))?;

    let node = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    super::step::replay_step(node, &mut resume_run, &mut resume_store, &plan)
        .map_err(replay_err_to_core)?;

    assert_eq!(*resume_run.read_slot(SlotIdx::new(2))?, SlotValue::I64(30));
    assert_eq!(
        *resume_run.read_slot(SlotIdx::new(2))?,
        *frame_full.read_slot(SlotIdx::new(2))?,
        "intermediate snapshot ran ahead of full replay"
    );
    Ok(())
}

// =========================================================================
// i64 extremes boundary tests
// =========================================================================

#[test]
fn replay_choose_expr_i64_max_boundary_expression() -> Result<(), CoreError> {
    let expr_gt = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Gt,
        ]
        .into(),
    )?;
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
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
        vec![ConstValue::I64(42), ConstValue::I64(0)],
        vec![expr_gt],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(i64::MAX))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(i64::MAX - 1))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(1));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "i64::MAX > i64::MAX-1 should be true",
        }),
    }
}

#[test]
fn replay_choose_expr_i64_min_boundary_expression() -> Result<(), CoreError> {
    let expr_lt = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Lt,
        ]
        .into(),
    )?;
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
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
        vec![ConstValue::I64(42), ConstValue::I64(0)],
        vec![expr_lt],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(i64::MIN))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(i64::MIN + 1))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(1));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "i64::MIN < i64::MIN+1 should be true",
        }),
    }
}

#[test]
fn replay_setconst_with_i64_max_value() -> Result<(), CoreError> {
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
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(i64::MAX)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(0), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(*frame.read_slot(SlotIdx::new(0))?, SlotValue::I64(i64::MAX));
    Ok(())
}

#[test]
fn replay_setconst_with_i64_min_value() -> Result<(), CoreError> {
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
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(i64::MIN)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(0), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(*frame.read_slot(SlotIdx::new(0))?, SlotValue::I64(i64::MIN));
    Ok(())
}

#[test]
fn replay_setconst_with_i64_zero() -> Result<(), CoreError> {
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
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(0), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(*frame.read_slot(SlotIdx::new(0))?, SlotValue::I64(0));
    Ok(())
}
