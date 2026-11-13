//! Shipping acceptance: checked ConcurrentV3 constructor with exact cached
//! frame/group requirement oracles. Validates the full constructor pipeline
//! for the five concurrent node kinds.

use vb_core::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, WorkflowError, WorkflowIrProfile, WorkflowParts, node,
};

fn n(
    id: u16,
    kind: CompiledNodeKind,
    out: Option<u16>,
    next: Option<u16>,
) -> vb_core::workflow::CompiledNode {
    node(
        StepIdx::new(id),
        kind,
        out.map(SlotIdx::new),
        next.map(StepIdx::new),
    )
}

fn parts(name: &str, nodes: Box<[vb_core::workflow::CompiledNode]>, slots: u16) -> WorkflowParts {
    let step_names = nodes.iter().map(|_| "step".into()).collect();
    WorkflowParts {
        name: name.into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: slots,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names,
        input_slots: Box::new([]),
    }
}

#[test]
fn together_concurrent_constructor_acceptance() -> Result<(), WorkflowError> {
    let start = n(
        0,
        CompiledNodeKind::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(3),
        },
        None,
        None,
    );
    let lane = n(1, CompiledNodeKind::Nop, None, Some(2));
    let lane_end = n(
        2,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        None,
        Some(3),
    );
    let join = n(
        3,
        CompiledNodeKind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(0),
        Some(4),
    );
    let finish = n(
        4,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );

    let wf = CompiledWorkflow::try_from_concurrent_parts(parts(
        "together",
        Box::new([start, lane, lane_end, join, finish]),
        1,
    ))?;
    assert_eq!(wf.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert_eq!(
        wf.concurrent_requirements()
            .map(|r| (r.frames(), r.groups())),
        Some((2, 1))
    );
    Ok(())
}

#[test]
fn first_success_constructor_acceptance() -> Result<(), WorkflowError> {
    let start = n(
        0,
        CompiledNodeKind::FirstSuccessStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(3),
        },
        None,
        None,
    );
    let lane = n(
        1,
        CompiledNodeKind::BuildList {
            items: Box::new([]),
        },
        Some(0),
        Some(2),
    );
    let lane_end = n(
        2,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: Some(SlotIdx::new(0)),
        },
        None,
        Some(3),
    );
    let join = n(
        3,
        CompiledNodeKind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(0),
        Some(4),
    );
    let finish = n(
        4,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );

    let wf = CompiledWorkflow::try_from_concurrent_parts(parts(
        "first_success",
        Box::new([start, lane, lane_end, join, finish]),
        1,
    ))?;
    assert_eq!(wf.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert_eq!(
        wf.concurrent_requirements()
            .map(|r| (r.frames(), r.groups())),
        Some((2, 1))
    );
    Ok(())
}

#[test]
fn foreach_concurrent_constructor_acceptance() -> Result<(), WorkflowError> {
    let start = n(
        0,
        CompiledNodeKind::ForEachConcurrentStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: FanoutLimit::new(2),
            at_once: BranchCount::new(2),
            body: StepIdx::new(1),
            join: StepIdx::new(3),
        },
        None,
        None,
    );
    let lane = n(1, CompiledNodeKind::Nop, None, Some(2));
    let lane_end = n(
        2,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        None,
        Some(3),
    );
    let join = n(
        3,
        CompiledNodeKind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(1),
        Some(4),
    );
    let finish = n(
        4,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    );

    let mut p = parts(
        "foreach",
        Box::new([start, lane, lane_end, join, finish]),
        2,
    );
    p.input_slots = Box::new([CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::List,
        constraint: CompiledInputConstraint::required(),
    }]);

    let wf = CompiledWorkflow::try_from_concurrent_parts(p)?;
    assert_eq!(wf.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert_eq!(
        wf.concurrent_requirements()
            .map(|r| (r.frames(), r.groups())),
        Some((3, 1))
    );
    Ok(())
}

#[test]
fn legacy_constructor_still_rejects_concurrent_kinds() {
    let start = n(
        0,
        CompiledNodeKind::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(3),
        },
        None,
        None,
    );
    let lane = n(1, CompiledNodeKind::Nop, None, Some(2));
    let lane_end = n(
        2,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        None,
        Some(3),
    );
    let join = n(
        3,
        CompiledNodeKind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(0),
        Some(4),
    );
    let finish = n(
        4,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );

    let p = parts("legacy", Box::new([start, lane, lane_end, join, finish]), 1);
    let result = CompiledWorkflow::try_from_parts(p).map(|_| ());
    assert_eq!(
        result,
        Err(WorkflowError::ConcurrentProfileRequired {
            step: StepIdx::new(0)
        })
    );
}

#[test]
fn checked_ctor_rejects_unpaired_lane_end() {
    let lane_end = n(
        0,
        CompiledNodeKind::ConcurrentLaneEnd {
            owner: StepIdx::new(99),
            result: None,
        },
        None,
        Some(1),
    );
    let finish = n(
        1,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );

    let p = parts("unpaired", Box::new([lane_end, finish]), 1);
    let result = CompiledWorkflow::try_from_concurrent_parts(p).map(|_| ());
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99)
        })
    );
}
