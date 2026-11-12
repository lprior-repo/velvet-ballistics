use vb_core::frame::RunFrame;
use vb_core::replay::{ReplayAction, ReplayError, replay_step};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, SlotBranch, WorkflowParts, node,
};
use vb_core::{RunId, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn frame(steps: u16, slots: u16, executed: u64) -> TestResult<RunFrame> {
    let mut run = RunFrame::new(RunId::new(93), StepIdx::ZERO, steps, slots)?;
    run.mark_running(StepIdx::ZERO)?;
    run.set_max_parallel_in_flight(7);
    run.add_parallel_in_flight(1)?;
    let mut checkpoint = run.checkpoint_v1();
    checkpoint.executed = executed;
    Ok(RunFrame::from_checkpoint_v1(checkpoint).map_err(|error| format!("{error:?}"))?)
}

pub(super) fn plan(otherwise: bool) -> TestResult<CompiledWorkflow> {
    let choice = node(
        StepIdx::ZERO,
        CompiledNodeKind::ChooseSlot {
            branches: Box::new([
                SlotBranch {
                    condition: SlotIdx::new(0),
                    target: StepIdx::new(1),
                },
                SlotBranch {
                    condition: SlotIdx::new(1),
                    target: StepIdx::new(2),
                },
            ]),
            otherwise: otherwise.then_some(StepIdx::new(3)).into(),
        },
        None,
        None,
    );
    let terminal = |id| node(StepIdx::new(id), CompiledNodeKind::Nop, None, None);
    let nodes: Box<[_]> = if otherwise {
        Box::new([choice, terminal(1), terminal(2), terminal(3)])
    } else {
        Box::new([choice, terminal(1), terminal(2)])
    };
    let input = |slot| CompiledInputSlot {
        slot: SlotIdx::new(slot),
        kind: InputSlotKind::Any,
        constraint: CompiledInputConstraint {
            presence: false.into(),
            ..CompiledInputConstraint::unconstrained()
        },
    };
    Ok(CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: "checked_frame_boundaries".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([input(0), input(1)]),
    })?)
}

pub(super) fn conditions(
    run: &mut RunFrame,
    a: Option<SlotValue>,
    b: Option<SlotValue>,
) -> TestResult {
    if let Some(value) = a {
        run.write_slot_with_taint(SlotIdx::new(0), value, Taint::Secret)?;
    }
    if let Some(value) = b {
        run.write_slot_with_taint(SlotIdx::new(1), value, Taint::TimeDependent)?;
    }
    Ok(())
}

pub(super) fn dispatch(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
) -> TestResult<Result<StepIdx, ReplayError>> {
    let node = plan.node(StepIdx::ZERO).ok_or("missing checked choice")?;
    let mut store = ValueStore::new();
    let _symbol = store.insert_symbol("preserved boundary store")?;
    let before = store.clone();
    let action = replay_step(node, run, &mut store, plan);
    assert_eq!(store, before);
    match action {
        Ok(ReplayAction::Continue(target)) => Ok(Ok(target)),
        Err(error) => Ok(Err(error)),
        _ => Err("unexpected replay action".into()),
    }
}
