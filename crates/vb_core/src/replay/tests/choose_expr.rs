// =========================================================================
// Choose expression replay tests
// =========================================================================

#[test]
fn replay_choose_expr_false_takes_otherwise() -> Result<(), CoreError> {
    let expr_false = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into())?;
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(2),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(3)),
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
        ],
        vec![
            ConstValue::Bool(true),
            ConstValue::Bool(false),
            ConstValue::I64(0),
        ],
        vec![expr_false],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame.pc(), StepIdx::new(3));
    Ok(())
}

#[test]
fn replay_choose_expr_first_true_branch_wins_over_second() -> Result<(), CoreError> {
    let expr_true = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into())?;
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(2),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Choose {
                    branches: vec![
                        ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(2),
                        },
                        ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(3),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(4)),
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
        vec![
            ConstValue::Bool(true),
            ConstValue::Bool(false),
            ConstValue::I64(0),
        ],
        vec![
            expr_true.clone(),
            ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into())?,
        ],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame.pc(), StepIdx::new(2));
    Ok(())
}

#[test]
fn replay_choose_expr_empty_branches_with_otherwise_uses_fallback() -> Result<(), CoreError> {
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
                CompiledNodeKind::Choose {
                    branches: vec![].into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
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
        vec![ConstValue::I64(42)],
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
fn replay_choose_expr_empty_branches_no_otherwise_returns_error() -> Result<(), CoreError> {
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
                CompiledNodeKind::Choose {
                    branches: vec![].into_boxed_slice(),
                    otherwise: None,
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
        vec![ConstValue::I64(42)],
        vec![],
    );
    match result {
        Err(CoreError::InvalidCompiledWorkflow { reason }) => {
            assert_eq!(reason, "test workflow validation failed");
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "expected InvalidCompiledWorkflow from validation",
        }),
    }
}

#[test]
fn replay_choose_expr_all_false_no_otherwise_returns_error() -> Result<(), CoreError> {
    let expr_false = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into())?;
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(2),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: None,
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
        vec![
            ConstValue::Bool(true),
            ConstValue::Bool(false),
            ConstValue::I64(0),
        ],
        vec![expr_false],
    )?;

    let mut store = ValueStore::new();
    match ReplayEngine::new(&plan).replay_frame_through(StepIdx::new(1), &mut store) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "choose_expr no branch matched and no otherwise");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected all-false no-otherwise error",
        }),
    }
}

#[test]
fn replay_choose_expr_gt_comparison_true_branch() -> Result<(), CoreError> {
    let expr_gt = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Gt,
        ]
        .into(),
    )?;
    let plan = make_plan_with_second_input(
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
        vec![ConstValue::I64(1), ConstValue::I64(0)],
        vec![expr_gt],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(10))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(5))?;

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
            assert_eq!(run.pc(), StepIdx::new(1));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "GT comparison true should take branch to step 1",
        }),
    }
}

#[test]
fn replay_choose_expr_gt_comparison_false_takes_otherwise() -> Result<(), CoreError> {
    let expr_gt = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Gt,
        ]
        .into(),
    )?;
    let plan = make_plan_with_second_input(
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
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
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
        vec![],
        vec![expr_gt],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(3))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(8))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action =
        super::step::replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) => {
            assert_eq!(next, StepIdx::new(2));
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "GT comparison false should take otherwise",
        }),
    }
}

#[test]
fn replay_choose_expr_non_boolean_condition_returns_error() -> Result<(), CoreError> {
    let expr_i64 = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into())?;
    let plan = make_rejected_plan(
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
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
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
        vec![ConstValue::I64(42)],
        vec![expr_i64],
        crate::workflow::SlotFlowFailure::IncompatibleType,
    )?;

    let mut store = ValueStore::new();
    match ReplayEngine::new(&plan).replay_frame_through(StepIdx::new(0), &mut store) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "choose_expr condition is not boolean");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected non-bool condition error",
        }),
    }
}

#[test]
fn replay_choose_expr_equality_predicate_true() -> Result<(), CoreError> {
    let expr_eq = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Eq,
        ]
        .into(),
    )?;
    let plan = make_plan_with_second_input(
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
        vec![ConstValue::I64(7), ConstValue::I64(0)],
        vec![expr_eq],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(7))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(7))?;

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
            reason: "Eq(7,7) should be true, take branch",
        }),
    }
}

#[test]
fn replay_choose_expr_and_combinator_both_true_takes_branch() -> Result<(), CoreError> {
    let expr_and = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::And,
        ]
        .into(),
    )?;
    let plan = make_plan_with_second_input(
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
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
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
        vec![],
        vec![expr_and],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))?;

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
            reason: "true && true should take branch",
        }),
    }
}

#[test]
fn replay_choose_expr_or_combinator_one_true_takes_branch() -> Result<(), CoreError> {
    let expr_or = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Or,
        ]
        .into(),
    )?;
    let plan = make_plan_with_second_input(
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
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
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
        vec![],
        vec![expr_or],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))?;
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))?;

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
            reason: "false || true should take branch",
        }),
    }
}

#[test]
fn replay_choose_expr_three_branches_second_true_wins() -> Result<(), CoreError> {
    let expr_true = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into())?;
    let expr_false_a = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(2))].into())?;
    let expr_false_b = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(2))].into())?;
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
                CompiledNodeKind::Choose {
                    branches: vec![
                        ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(2),
                        },
                        ExprBranch {
                            condition: ExprIdx::new(1),
                            target: StepIdx::new(3),
                        },
                        ExprBranch {
                            condition: ExprIdx::new(2),
                            target: StepIdx::new(4),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(5)),
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
            crate::workflow::node(
                StepIdx::new(5),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![
            ConstValue::I64(0),
            ConstValue::Bool(true),
            ConstValue::Bool(false),
        ],
        vec![expr_false_a, expr_true, expr_false_b],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame.pc(), StepIdx::new(3));
    Ok(())
}

