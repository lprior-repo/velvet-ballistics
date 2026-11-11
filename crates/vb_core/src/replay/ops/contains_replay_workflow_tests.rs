enum ContainsReplayReceiver {
    List,
    Symbol,
}

struct ContainsReplayFixture {
    plan: crate::workflow::CompiledWorkflow,
    run: RunFrame,
    store: ValueStore,
}

fn contains_replay_workflow_nodes() -> Box<[CompiledNode]> {
    vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(1)),
        ),
        contains_replay_choose_node(),
        contains_replay_finish_node(StepIdx::new(2)),
        contains_replay_finish_node(StepIdx::new(3)),
    ]
    .into_boxed_slice()
}

fn contains_replay_choose_node() -> CompiledNode {
    let target = StepIdx::new(2);
    node(
        StepIdx::new(1),
        CompiledNodeKind::Choose {
            branches: vec![crate::workflow::ExprBranch {
                condition: ExprIdx::new(0),
                target,
            }]
            .into_boxed_slice(),
            otherwise: Some(StepIdx::new(3)).into(),
        },
        None,
        Some(target),
    )
}

fn contains_replay_finish_node(step: StepIdx) -> CompiledNode {
    node(
        step,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )
}

fn contains_replay_workflow() -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    let expression = make_expr_program(vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Contains,
    ])?;
    make_plan(
        contains_replay_workflow_nodes().into_vec(),
        vec![],
        vec![expression],
        vec![],
    )
}

fn contains_replay_fixture(
    receiver: ContainsReplayReceiver,
    items: Vec<SlotValue>,
    needle: SlotValue,
) -> Result<ContainsReplayFixture, CoreError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(items.into_boxed_slice())?;
    let symbol = store.insert_symbol("valid receiver")?;
    let receiver = match receiver {
        ContainsReplayReceiver::List => SlotValue::List(list),
        ContainsReplayReceiver::Symbol => SlotValue::Symbol(symbol),
    };
    let plan = contains_replay_workflow()?;
    let mut run = RunFrame::new(
        RunId::new(1),
        StepIdx::new(1),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), receiver)?;
    run.write_slot(SlotIdx::new(1), needle)?;
    Ok(ContainsReplayFixture { plan, run, store })
}

fn contains_replay_workflow_step(
    fixture: &mut ContainsReplayFixture,
) -> Result<crate::replay::step::ReplayAction, ReplayError> {
    let step = StepIdx::new(1);
    let node = fixture
        .plan
        .node(step)
        .ok_or(ReplayError::StepNotFound { step })?;
    crate::replay::step::replay_step(node, &mut fixture.run, &mut fixture.store, &fixture.plan)
}

#[test]
fn contains_replay_workflow_takes_true_branch() -> Result<(), CoreError> {
    let mut fixture = contains_replay_fixture(
        ContainsReplayReceiver::List,
        vec![SlotValue::I64(7), SlotValue::I64(11)],
        SlotValue::I64(11),
    )?;
    assert!(matches!(
        contains_replay_workflow_step(&mut fixture),
        Ok(crate::replay::step::ReplayAction::Continue(target)) if target == StepIdx::new(2)
    ));
    assert_eq!(fixture.run.pc(), StepIdx::new(2));
    Ok(())
}

#[test]
fn contains_replay_workflow_takes_otherwise_for_missing_item() -> Result<(), CoreError> {
    let mut fixture = contains_replay_fixture(
        ContainsReplayReceiver::List,
        vec![SlotValue::I64(7), SlotValue::I64(11)],
        SlotValue::I64(13),
    )?;
    assert!(matches!(
        contains_replay_workflow_step(&mut fixture),
        Ok(crate::replay::step::ReplayAction::Continue(target)) if target == StepIdx::new(3)
    ));
    assert_eq!(fixture.run.pc(), StepIdx::new(3));
    Ok(())
}

#[test]
fn contains_replay_workflow_takes_otherwise_for_empty_list() -> Result<(), CoreError> {
    let mut fixture =
        contains_replay_fixture(ContainsReplayReceiver::List, Vec::new(), SlotValue::I64(7))?;
    assert!(matches!(
        contains_replay_workflow_step(&mut fixture),
        Ok(crate::replay::step::ReplayAction::Continue(target)) if target == StepIdx::new(3)
    ));
    assert_eq!(fixture.run.pc(), StepIdx::new(3));
    Ok(())
}

#[test]
fn contains_replay_workflow_wraps_wrong_receiver_at_choose_pc() -> Result<(), CoreError> {
    let mut fixture = contains_replay_fixture(
        ContainsReplayReceiver::Symbol,
        vec![SlotValue::I64(7)],
        SlotValue::I64(7),
    )?;
    assert!(matches!(
        contains_replay_workflow_step(&mut fixture),
        Err(ReplayError::ExpressionEvalFailed { step }) if step == StepIdx::new(1)
    ));
    assert_eq!(fixture.run.pc(), StepIdx::new(1));
    Ok(())
}
