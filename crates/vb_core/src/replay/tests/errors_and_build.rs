#[test]
fn ce_004_replay_invalid_program_counter_yields_typed_step_not_found() -> Result<(), CoreError> {
    // CE-004 regression: invalid jump/branch targets produced by `run.set_pc`
    // must surface as `ReplayError::StepNotFound { step }`, not collapse to
    // the generic `Internal` variant. Reserve `Internal` for genuinely
    // unexpected engine failures.
    let invalid = super::engine_to_replay_err(CoreError::InvalidProgramCounter {
        step: StepIdx::new(99),
    });
    assert_eq!(
        invalid,
        ReplayError::StepNotFound {
            step: StepIdx::new(99),
        },
        "CE-004: invalid jump target must map to typed StepNotFound"
    );
    Ok(())
}

#[test]
fn ce_004_engine_to_replay_err_reserves_internal_for_unexpected_failures() -> Result<(), CoreError>
{
    // CE-004 boundary: the converter must continue to route genuinely
    // unexpected engine errors to `Internal` and must not silently
    // promote them to `StepNotFound` or `SlotNotAvailable`.
    let budget = super::engine_to_replay_err(CoreError::StepBudgetExhausted);
    assert_eq!(
        budget,
        ReplayError::Internal {
            reason: "unexpected engine error during replay",
        },
        "CE-004: non-slot, non-PC engine errors must still map to Internal"
    );
    let slot = super::engine_to_replay_err(CoreError::SlotOutOfBounds {
        slot: SlotIdx::new(3),
    });
    assert_eq!(
        slot,
        ReplayError::SlotNotAvailable {
            slot: SlotIdx::new(3),
        },
        "CE-004: slot errors must continue to map to SlotNotAvailable"
    );
    Ok(())
}

#[test]
fn replay_slot_error_conversion_preserves_exact_slot_variants() -> Result<(), CoreError> {
    let out_of_bounds = super::engine_to_replay_err(CoreError::SlotOutOfBounds {
        slot: SlotIdx::new(6),
    });
    let uninitialized = super::engine_to_replay_err(CoreError::SlotUninitialized {
        slot: SlotIdx::new(7),
    });
    let other = super::engine_to_replay_err(CoreError::StepBudgetExhausted);

    assert_eq!(
        out_of_bounds,
        ReplayError::SlotNotAvailable {
            slot: SlotIdx::new(6)
        }
    );
    assert_eq!(
        uninitialized,
        ReplayError::SlotNotAvailable {
            slot: SlotIdx::new(7)
        }
    );
    assert_eq!(
        other,
        ReplayError::Internal {
            reason: "unexpected engine error during replay"
        }
    );
    Ok(())
}

#[test]
fn replay_choose_expr_takes_true_branch() -> Result<(), CoreError> {
    let expr = ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into())?;
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
                    value: ConstIdx::new(1),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(2),
                },
                Some(SlotIdx::new(0)),
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
        vec![
            ConstValue::Bool(true),
            ConstValue::I64(1),
            ConstValue::I64(2),
        ],
        vec![expr],
    )?;
    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(0), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(frame.pc(), StepIdx::new(1));
    Ok(())
}

#[test]
fn replay_collect_start_rejects_non_list_source() -> Result<(), CoreError> {
    let plan = collect_invalid_start_plan(4, 1)?;
    let mut run = RunFrame::new(
        RunId::new(95),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(10))?;

    match replay_step_error(&plan, &mut run, StepIdx::new(0)) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect slot was not list during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected non-list collect source error",
        }),
    }
}

#[test]
fn replay_collect_page_rejects_non_list_collector() -> Result<(), CoreError> {
    let plan = collect_page_finish_plan()?;
    let mut run = RunFrame::new(
        RunId::new(96),
        StepIdx::new(1),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(10))?;

    match replay_step_error(&plan, &mut run, StepIdx::new(1)) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect slot was not list during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected non-list collect page error",
        }),
    }
}

#[test]
fn replay_collect_start_rejects_missing_source_list() -> Result<(), CoreError> {
    let plan = collect_invalid_start_plan(4, 1)?;
    let mut run = RunFrame::new(
        RunId::new(97),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(ListId::new(99)))?;

    match replay_step_error(&plan, &mut run, StepIdx::new(0)) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect source list missing during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected missing collect source list error",
        }),
    }
}

#[test]
fn replay_collect_start_reports_page_insert_failure() -> Result<(), CoreError> {
    let plan = collect_invalid_start_plan(4, 1)?;
    let mut store = ValueStore::with_max_slots(1);
    let source = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test capped source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(98),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(source))?;
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    match super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "insert collect page failed during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected collect page insert failure",
        }),
    }
}

#[test]
fn replay_collect_next_rejects_missing_current_page() -> Result<(), CoreError> {
    let plan = collect_page_finish_plan()?;
    let mut run = RunFrame::new(
        RunId::new(99),
        StepIdx::new(2),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(1), SlotValue::List(ListId::new(99)))?;

    match replay_step_error(&plan, &mut run, StepIdx::new(2)) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect current page missing during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected missing collect current page error",
        }),
    }
}

#[test]
fn replay_collect_next_rejects_changed_source_length() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let (plan, mut run) = collect_source_frame(&mut store)?;
    let mut states = super::step::ReplayCollectStates::new();

    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(0))?;

    let current_page = match *run.read_slot(SlotIdx::new(1))? {
        SlotValue::List(list) => list,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "collector slot not list after collect start",
            });
        }
    };
    let mut changed_store = ValueStore::new();
    changed_store
        .insert_list(vec![SlotValue::I64(10), SlotValue::I64(20)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test changed source list insert failed",
        })?;
    let changed_current_page = changed_store
        .insert_list(vec![SlotValue::I64(10)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test current page list insert failed",
        })?;
    if changed_current_page != current_page {
        return Err(CoreError::InternalInvariantViolation {
            reason: "test current page handle mismatch",
        });
    }

    let node = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(2),
        })?;
    match super::step::replay_step_with_collect(
        node,
        &mut run,
        &mut changed_store,
        &plan,
        &mut states,
    ) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect source length changed during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected changed collect source length replay error",
        }),
    }
}

#[test]
fn replay_value_writers_report_missing_output_slots() -> Result<(), CoreError> {
    let expr = ExprProgram::try_from_ops(vec![ExprOp::LoadSlot(SlotIdx::new(0))].into())?;
    let plan = make_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )],
        vec![ConstValue::I64(7)],
        vec![expr],
    )?;
    let mut store = ValueStore::new();
    let mut states = super::step::ReplayCollectStates::new();

    let set_const = replay_node_error(
        &plan,
        &mut store,
        &mut states,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    )?;
    assert_eq!(set_const, "SetConst node missing output slot");

    let copy = replay_node_error(
        &plan,
        &mut store,
        &mut states,
        CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
    )?;
    assert_eq!(copy, "Copy node missing output slot");

    let eval_expr = replay_node_error(
        &plan,
        &mut store,
        &mut states,
        CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0),
        },
    )?;
    assert_eq!(eval_expr, "EvalExpr node missing output slot");

    let build_object = replay_node_error(
        &plan,
        &mut store,
        &mut states,
        CompiledNodeKind::BuildObject {
            fields: vec![(SymbolId::new(0), SlotIdx::new(0))].into_boxed_slice(),
        },
    )?;
    assert_eq!(build_object, "BuildObject node missing output slot");

    let build_list = replay_node_error(
        &plan,
        &mut store,
        &mut states,
        CompiledNodeKind::BuildList {
            items: vec![SlotIdx::new(0)].into_boxed_slice(),
        },
    )?;
    assert_eq!(build_list, "BuildList node missing output slot");
    Ok(())
}

fn replay_node_error(
    plan: &crate::workflow::CompiledWorkflow,
    store: &mut ValueStore,
    states: &mut super::step::ReplayCollectStates,
    kind: CompiledNodeKind,
) -> Result<&'static str, CoreError> {
    let node = crate::workflow::node(StepIdx::new(0), kind, None, Some(StepIdx::new(0)));
    let mut run = RunFrame::new(
        RunId::new(102),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(7))?;

    match super::step::replay_step_with_collect(&node, &mut run, store, plan, states) {
        Err(ReplayError::Internal { reason }) => Ok(reason),
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected missing output slot replay error",
        }),
    }
}

#[test]
fn replay_build_object_reports_insert_failure() -> Result<(), CoreError> {
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
    let mut run = RunFrame::new(
        RunId::new(100),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::with_max_slots(1);
    store
        .insert_list(Vec::new().into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test cap filler insert failed",
        })?;
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    match super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "insert_object failed");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected build object insert failure",
        }),
    }
}

#[test]
fn replay_build_list_reports_insert_failure() -> Result<(), CoreError> {
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
    let mut run = RunFrame::new(
        RunId::new(101),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::with_max_slots(1);
    store
        .insert_list(Vec::new().into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test cap filler insert failed",
        })?;
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    match super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states) {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "insert_list failed");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected build list insert failure",
        }),
    }
}

fn replay_step_error(
    plan: &crate::workflow::CompiledWorkflow,
    run: &mut RunFrame,
    step: StepIdx,
) -> Result<(), ReplayError> {
    let mut store = ValueStore::new();
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan.node(step).ok_or(ReplayError::StepNotFound { step })?;
    super::step::replay_step_with_collect(node, run, &mut store, plan, &mut states).map(|_| ())
}

