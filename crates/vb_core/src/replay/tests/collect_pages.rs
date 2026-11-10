fn collect_source_frame(
    store: &mut ValueStore,
) -> Result<(crate::workflow::CompiledWorkflow, RunFrame), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit: 4,
                    page_size: 1,
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::CollectNext {
                    collector_slot: SlotIdx::new(1),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::CollectFinish {
                    collector_slot: SlotIdx::new(1),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(4)),
            ),
            crate::workflow::node(
                StepIdx::new(4),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
    )?;
    let source = store
        .insert_list(
            vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(88),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(source), Taint::Secret)?;
    Ok((plan, run))
}

fn current_page_values(run: &RunFrame, store: &ValueStore) -> Result<Box<[SlotValue]>, CoreError> {
    let list = match *run.read_slot(SlotIdx::new(1))? {
        SlotValue::List(list) => list,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "collector slot not list",
            });
        }
    };
    store
        .list(list)
        .map(|items| items.to_vec().into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "collector list missing",
        })
}

struct PageWithTaint {
    values: Box<[SlotValue]>,
    taint: Taint,
}

struct ReplayCollectPages {
    first: PageWithTaint,
    second: PageWithTaint,
    third: PageWithTaint,
}

#[test]
fn replay_collect_state_restores_in_flight_ordering_and_taint() {
    match replay_collect_pages_with_taint() {
        Ok(pages) => {
            assert_eq!(&*pages.first.values, &[SlotValue::I64(10)]);
            assert_eq!(pages.first.taint, Taint::Secret);
            assert_eq!(&*pages.second.values, &[SlotValue::I64(20)]);
            assert_eq!(pages.second.taint, Taint::Secret);
            assert_eq!(&*pages.third.values, &[SlotValue::I64(30)]);
            assert_eq!(pages.third.taint, Taint::Secret);
        }
        Err(err) => {
            panic!("collect replay state failed: {err:?}");
        }
    }
}

fn replay_collect_pages_with_taint() -> Result<ReplayCollectPages, CoreError> {
    let mut store = ValueStore::new();
    let (plan, mut run) = collect_source_frame(&mut store)?;
    let mut states = super::step::ReplayCollectStates::new();
    let start = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;
    let next = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(2),
        })?;

    super::step::replay_step_with_collect(start, &mut run, &mut store, &plan, &mut states)
        .map_err(replay_err_to_core)?;
    let first = PageWithTaint {
        values: current_page_values(&run, &store)?,
        taint: run.read_taint(SlotIdx::new(1))?,
    };

    super::step::replay_step_with_collect(next, &mut run, &mut store, &plan, &mut states)
        .map_err(replay_err_to_core)?;
    let second = PageWithTaint {
        values: current_page_values(&run, &store)?,
        taint: run.read_taint(SlotIdx::new(1))?,
    };

    super::step::replay_step_with_collect(next, &mut run, &mut store, &plan, &mut states)
        .map_err(replay_err_to_core)?;
    let third = PageWithTaint {
        values: current_page_values(&run, &store)?,
        taint: run.read_taint(SlotIdx::new(1))?,
    };
    Ok(ReplayCollectPages {
        first,
        second,
        third,
    })
}

#[test]
fn replay_collect_next_without_hydrated_state_reports_typed_error() {
    match replay_collect_next_without_hydrated_state() {
        Ok(Err(ReplayError::Internal { reason })) => {
            assert_eq!(reason, "collect pagination state missing during replay");
        }
        other => assert!(
            matches!(other, Ok(Err(ReplayError::Internal { .. }))),
            "unexpected collect replay result: {other:?}"
        ),
    }
}

fn replay_collect_next_without_hydrated_state() -> Result<Result<(), ReplayError>, CoreError> {
    let mut store = ValueStore::new();
    let (plan, mut run) = collect_source_frame(&mut store)?;
    let mut states = super::step::ReplayCollectStates::new();
    let start = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;
    let next = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(2),
        })?;

    super::step::replay_step_with_collect(start, &mut run, &mut store, &plan, &mut states)
        .map_err(replay_err_to_core)?;
    let mut empty_states = super::step::ReplayCollectStates::new();
    Ok(
        super::step::replay_step_with_collect(next, &mut run, &mut store, &plan, &mut empty_states)
            .map(|_| ()),
    )
}

#[test]
fn replay_collect_start_page_bound_reports_exact_error() {
    match replay_collect_start_page_bound() {
        Ok(Err(ReplayError::Internal { reason })) => {
            assert_eq!(reason, "collect page size exceeds limit during replay");
        }
        other => assert!(
            matches!(other, Ok(Err(ReplayError::Internal { .. }))),
            "unexpected collect replay result: {other:?}"
        ),
    }
}

fn replay_collect_start_page_bound() -> Result<Result<(), ReplayError>, CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit: 1,
                    page_size: 2,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                Some(SlotIdx::new(1)),
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
        vec![],
    )?;
    let mut store = ValueStore::new();
    let source = store
        .insert_list(vec![SlotValue::I64(10)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(89),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(source))?;
    let mut states = super::step::ReplayCollectStates::new();
    let start = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    Ok(
        super::step::replay_step_with_collect(start, &mut run, &mut store, &plan, &mut states)
            .map(|_| ()),
    )
}

#[test]
fn replay_frame_through_executes_target_step() -> Result<(), CoreError> {
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
        vec![ConstValue::I64(64)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(&plan)
        .replay_frame_through(StepIdx::new(0), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(*frame.read_slot(SlotIdx::new(0))?, SlotValue::I64(64));
    assert_eq!(frame.pc(), StepIdx::new(1));
    Ok(())
}

#[test]
fn replay_before_target_stops_when_finish_returns_none() -> Result<(), CoreError> {
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
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        ],
        vec![ConstValue::I64(11)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let reached = ReplayEngine::new(&plan)
        .replay_up_to(StepIdx::new(2), &mut store)
        .map_err(replay_err_to_core)?;

    assert_eq!(reached, StepIdx::new(1));
    Ok(())
}

#[test]
fn replay_collect_page_finish_and_empty_next_paths() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let plan = collect_page_finish_plan()?;
    let source = store
        .insert_list(
            vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(90),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(source), Taint::Secret)?;

    let mut states = super::step::ReplayCollectStates::new();
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(0))?;
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(1))?;
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(2))?;
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(1))?;
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(2))?;
    let prior_pc = run.pc();
    let prior_collector = *run.read_slot(SlotIdx::new(1))?;
    assert_eq!(prior_pc, StepIdx::new(3));
    let next = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(2),
        })?;
    let rejected =
        super::step::replay_step_with_collect(next, &mut run, &mut store, &plan, &mut states);
    assert!(matches!(
        rejected,
        Err(ReplayError::Internal {
            reason: "collect pagination state missing during replay",
        })
    ));
    assert_eq!(run.pc(), prior_pc);
    assert_eq!(*run.read_slot(SlotIdx::new(1))?, prior_collector);
    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(3))?;

    assert_eq!(run.pc(), StepIdx::new(4));
    assert_eq!(run.read_taint(SlotIdx::new(2))?, Taint::Secret);
    assert_eq!(
        &*current_page_values(&run, &store)?,
        &[SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)]
    );
    assert_eq!(*run.read_slot(SlotIdx::new(1))?, SlotValue::List(source));
    assert_eq!(*run.read_slot(SlotIdx::new(2))?, SlotValue::List(source));
    assert_eq!(run.read_taint(SlotIdx::new(1))?, Taint::Secret);
    Ok(())
}

fn collect_page_finish_plan() -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit: 4,
                    page_size: 2,
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::CollectPage {
                    collector_slot: SlotIdx::new(1),
                    body: StepIdx::new(2),
                    done: StepIdx::new(3),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::CollectNext {
                    collector_slot: SlotIdx::new(1),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::CollectFinish {
                    collector_slot: SlotIdx::new(1),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(4)),
            ),
            crate::workflow::node(
                StepIdx::new(4),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
    )
}

fn replay_plan_step(
    plan: &crate::workflow::CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut super::step::ReplayCollectStates,
    step: StepIdx,
) -> Result<(), CoreError> {
    let node = plan
        .node(step)
        .ok_or(CoreError::InvalidProgramCounter { step })?;
    super::step::replay_step_with_collect(node, run, store, plan, states)
        .map(|_| ())
        .map_err(replay_err_to_core)
}

#[test]
fn replay_collect_start_empty_source_jumps_done() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let (plan, mut run) = collect_empty_source_frame(&mut store)?;
    let mut states = super::step::ReplayCollectStates::new();

    replay_plan_step(&plan, &mut run, &mut store, &mut states, StepIdx::new(0))?;

    assert_eq!(run.pc(), StepIdx::new(2));
    assert_eq!(&*current_page_values(&run, &store)?, &[]);
    Ok(())
}

fn collect_empty_source_frame(
    store: &mut ValueStore,
) -> Result<(crate::workflow::CompiledWorkflow, RunFrame), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit: 4,
                    page_size: 1,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
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
        vec![],
        vec![],
    )?;
    let source = store
        .insert_list(Vec::new().into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test empty list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(91),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(source))?;
    Ok((plan, run))
}

#[test]
fn replay_collect_start_rejects_zero_page_size() -> Result<(), CoreError> {
    match replay_collect_start_zero_page_size()? {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect page size was zero during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected zero page size replay error",
        }),
    }
}

fn replay_collect_start_zero_page_size() -> Result<Result<(), ReplayError>, CoreError> {
    let plan = collect_invalid_start_plan(4, 0)?;
    let mut store = ValueStore::new();
    let source = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(92),
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
    Ok(
        super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states)
            .map(|_| ()),
    )
}

#[test]
fn replay_collect_start_rejects_source_over_limit() -> Result<(), CoreError> {
    match replay_collect_start_source_over_limit()? {
        Err(ReplayError::Internal { reason }) => {
            assert_eq!(reason, "collect item count exceeds limit during replay");
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(()) => Err(CoreError::InternalInvariantViolation {
            reason: "expected source over limit replay error",
        }),
    }
}

fn replay_collect_start_source_over_limit() -> Result<Result<(), ReplayError>, CoreError> {
    let plan = collect_invalid_start_plan(1, 1)?;
    let mut store = ValueStore::new();
    let source = store
        .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "test source list insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(93),
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
    Ok(
        super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states)
            .map(|_| ()),
    )
}

fn collect_invalid_start_plan(
    limit: u32,
    page_size: u32,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit,
                    page_size,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
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
        vec![],
        vec![],
    )
}

#[test]
fn replay_error_handler_node_routes_to_body() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(1),
                    handler: StepIdx::new(2),
                    error_slot: Some(SlotIdx::new(0)),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(1), ConstValue::I64(2)],
        vec![],
    )?;
    let mut run = RunFrame::new(
        RunId::new(94),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();
    let mut states = super::step::ReplayCollectStates::new();
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    match super::step::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states) {
        Ok(ReplayAction::Continue(next)) => {
            assert_eq!(next, StepIdx::new(1));
            Ok(())
        }
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected ErrorHandler replay to continue to body",
        }),
        Err(other) => Err(replay_err_to_core(other)),
    }
}

