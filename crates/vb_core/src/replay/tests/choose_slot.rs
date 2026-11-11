// =========================================================================
// Choose slot replay tests
// =========================================================================

#[test]
fn replay_choose_slot_first_true_branch_wins() -> Result<(), CoreError> {
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
                CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(2),
                        },
                        SlotBranch {
                            condition: SlotIdx::new(1),
                            target: StepIdx::new(3),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(4)).into(),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(4),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::Bool(true)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))?;

    let node = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(2));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "first true slot branch should win".into(),
        }),
    }
}

#[test]
fn replay_choose_slot_second_true_first_false() -> Result<(), CoreError> {
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
                CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(2),
                        },
                        SlotBranch {
                            condition: SlotIdx::new(1),
                            target: StepIdx::new(3),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(4)).into(),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(4),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::Bool(false)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))?;

    let node = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(3));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "second true slot branch should win after first false".into(),
        }),
    }
}

#[test]
fn replay_choose_slot_all_false_with_otherwise_fallback() -> Result<(), CoreError> {
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
                CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(2),
                        },
                        SlotBranch {
                            condition: SlotIdx::new(1),
                            target: StepIdx::new(3),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(4)).into(),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(4),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::Bool(false)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(false))?;

    let node = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(4));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "all false with otherwise should fallback".into(),
        }),
    }
}

#[test]
fn replay_choose_slot_all_false_no_otherwise_returns_error() -> Result<(), CoreError> {
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
                CompiledNodeKind::ChooseSlot {
                    branches: vec![SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: None.into(),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::Bool(false)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))?;

    let node = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    match super::step::replay_step(node, &mut run, &mut store, &plan) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "choose_slot no branch matched and no otherwise");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected all-false no-otherwise error".into(),
        }),
    }
}

#[test]
fn replay_choose_slot_empty_branches_with_otherwise_fallback() -> Result<(), CoreError> {
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
                CompiledNodeKind::ChooseSlot {
                    branches: vec![].into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)).into(),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
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
        .replay_frame_through(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame.pc(), StepIdx::new(2));
    Ok(())
}

#[test]
fn replay_choose_slot_empty_branches_no_otherwise_error() -> Result<(), CoreError> {
    let result = make_plan(
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
                CompiledNodeKind::ChooseSlot {
                    branches: vec![].into_boxed_slice(),
                    otherwise: None.into(),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(0)],
        vec![],
    );
    match result {
        Err(CoreError::InvalidCompiledWorkflow { reason }) => {
            assert_eq!(reason, "test workflow validation failed");
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "expected InvalidCompiledWorkflow from validation".into(),
        }),
    }
}

