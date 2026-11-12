// ---- Finish step ----

#[test]
fn replay_finish_returns_finished_action() -> Result<(), CoreError> {
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
        vec![ConstValue::I64(99)],
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
    let action = replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Finished => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Finish should return Finished".into(),
            });
        }
    }
    if run.executed() != 2 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "executed should be 2".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_finish_uninitialized_result_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(5),
            },
            None,
            None,
        )],
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
        "Finish with uninitialized result must fail"
    );
    Ok(())
}

// ---- Jump step ----

#[test]
fn replay_jump_advances_pc_to_target() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Jump {
                    target: StepIdx::new(1),
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
        ],
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(0))?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing".into(),
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Jump should return Continue(1)".into(),
            });
        }
    }
    if run.pc() != StepIdx::new(1) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "PC should be at step 1".into(),
        });
    }
    Ok(())
}

#[test]
fn ce_004_replay_jump_out_of_bounds_yields_typed_step_not_found() -> Result<(), CoreError> {
    // CE-004 E2E regression: a Jump node whose target is outside the run
    // frame's step array must reach the caller as `ReplayError::StepNotFound`,
    // not collapse to the generic `Internal` variant.
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Jump {
                    target: StepIdx::new(1),
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
        ],
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(0))?;
    let _store = ValueStore::new();

    // Drive `replay_jump` with an out-of-bounds target (99) directly to
    // exercise the `set_pc` failure path without relying on workflow
    // validation rejecting the plan.
    let result = super::replay_jump(&mut run, StepIdx::new(99));
    match result {
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "replay_jump accepted out-of-bounds target".into(),
        }),
        Err(ReplayError::StepNotFound { step }) if step == StepIdx::new(99) => Ok(()),
        Err(_) => Err(CoreError::InternalInvariantViolation {
            reason: "CE-004: out-of-bounds Jump must surface as typed StepNotFound".into(),
        }),
    }
}

// ---- Suspend steps ----

#[test]
fn replay_do_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            None,
            None,
        )],
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
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::ActionPending => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Do should return Suspended(0, Do)".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_ask_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None.into(),
            },
            None,
            None,
        )],
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
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::AskPending => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Ask should return Suspended(0, Ask)".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_wait_until_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
            None,
            None,
        )],
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
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::WaitUntil => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "WaitUntil should return Suspended(0, WaitUntil)".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_wait_event_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None.into(),
            },
            None,
            None,
        )],
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
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::WaitEvent => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "WaitEvent should return Suspended(0, WaitEvent)".into(),
            });
        }
    }
    Ok(())
}

// ---- ChooseSlot step ----

#[test]
fn replay_choose_slot_true_branch_taken() -> Result<(), CoreError> {
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
                    .into(),
                    otherwise: Some(StepIdx::new(3)).into(),
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
        vec![ConstValue::Bool(true)],
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
    let action = replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(2) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ChooseSlot true should go to step 2".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_choose_slot_false_falls_to_otherwise() -> Result<(), CoreError> {
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
                    .into(),
                    otherwise: Some(StepIdx::new(3)).into(),
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
        vec![ConstValue::Bool(false)],
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
    let action = replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(3) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ChooseSlot false should go to otherwise (step 3)".into(),
            });
        }
    }
    Ok(())
}

// ---- Multi-step counter ----

#[test]
fn replay_multi_step_executed_counter() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(3)),
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
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(0))?;
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

    let node3 = plan
        .node(StepIdx::new(3))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 3 missing".into(),
        })?;
    replay_step(node3, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.executed() != 4 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "executed counter should be 4".into(),
        });
    }
    Ok(())
}
