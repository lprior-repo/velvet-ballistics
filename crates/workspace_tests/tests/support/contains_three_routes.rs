use vb_core::{
    CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprIdx, ExprOp, ExprProgram, ResourceContract,
    RunFrame, RunId, SlotIdx, SlotValue, StepIdx, Taint, ValueStore, WorkflowDigest, WorkflowParts,
    node,
};
use vb_expr::eval::ExprHelper;

pub(super) struct ContainsFixture {
    pub(super) plan: CompiledWorkflow,
    pub(super) store: ValueStore,
    pub(super) receiver: SlotValue,
}

pub(super) fn fixture_with_store(
    mut store: ValueStore,
    items: Vec<SlotValue>,
) -> Result<ContainsFixture, String> {
    let list = store
        .insert_list(items)
        .map_err(|error| format!("{error:?}"))?;
    let plan = contains_plan()?;
    Ok(ContainsFixture {
        plan,
        store,
        receiver: SlotValue::List(list),
    })
}

fn contains_plan() -> Result<CompiledWorkflow, String> {
    let expression = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Contains,
        ]
        .into_boxed_slice(),
    )
    .map_err(|error| format!("{error:?}"))?;
    let parts = contains_parts(expression);
    CompiledWorkflow::try_from_parts(parts).map_err(|error| format!("{error:?}"))
}

fn contains_parts(expression: ExprProgram) -> WorkflowParts {
    WorkflowParts {
        name: "contains-three-routes".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        nodes: contains_nodes(),
        expressions: vec![expression].into_boxed_slice(),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    }
}

fn contains_nodes() -> Box<[vb_core::CompiledNode]> {
    vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(1)),
        ),
        choose_node(),
        finish_node(StepIdx::new(2)),
        finish_node(StepIdx::new(3)),
    ]
    .into_boxed_slice()
}

fn choose_node() -> vb_core::CompiledNode {
    node(
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
        Some(StepIdx::new(2)),
    )
}

fn finish_node(step: StepIdx) -> vb_core::CompiledNode {
    node(
        step,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )
}

pub(super) fn frame(
    plan: &CompiledWorkflow,
    receiver: SlotValue,
    needle: SlotValue,
) -> Result<RunFrame, String> {
    let mut run = RunFrame::new(
        RunId::new(1),
        StepIdx::new(1),
        plan.node_count(),
        plan.slot_count(),
    )
    .map_err(|error| format!("{error:?}"))?;
    run.write_slot(SlotIdx::new(0), receiver)
        .map_err(|error| format!("{error:?}"))?;
    run.write_slot(SlotIdx::new(1), needle)
        .map_err(|error| format!("{error:?}"))?;
    Ok(run)
}

pub(super) fn assert_routes(
    fixture: &mut ContainsFixture,
    needle: SlotValue,
    expected: bool,
) -> Result<(), String> {
    let args = [fixture.receiver, needle];
    let direct =
        vb_expr::eval::eval_helper_with_store(ExprHelper::Contains, &args, &mut fixture.store);
    assert_eq!(direct, Ok(SlotValue::Bool(expected)));
    let hot_run = frame(&fixture.plan, fixture.receiver, needle)?;
    let hot = vb_core::engine::eval_expr_with_store(
        &fixture.plan,
        &hot_run,
        &mut fixture.store,
        ExprIdx::new(0),
    );
    assert_eq!(hot, Ok((SlotValue::Bool(expected), Taint::Clean)));
    assert_replay(fixture, needle, expected)
}

fn assert_replay(
    fixture: &mut ContainsFixture,
    needle: SlotValue,
    expected: bool,
) -> Result<(), String> {
    let mut run = frame(&fixture.plan, fixture.receiver, needle)?;
    let step = StepIdx::new(1);
    let node = fixture
        .plan
        .node(step)
        .ok_or_else(|| "missing choose node".to_owned())?;
    let replay =
        vb_core::replay::step::replay_step(node, &mut run, &mut fixture.store, &fixture.plan);
    let target = if expected {
        StepIdx::new(2)
    } else {
        StepIdx::new(3)
    };
    assert!(
        matches!(replay, Ok(vb_core::replay::ReplayAction::Continue(actual)) if actual == target)
    );
    assert_eq!(run.pc(), target);
    Ok(())
}
