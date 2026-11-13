//! Constructor barrier tests — malformed concurrent fixtures remain rejected.
//!
//! Each malformed fixture asserts the exact intended structural error.
//! Shared bootstrap fixtures have no actual LaneEnd node; the join target
//! is step 2 (Nop), not a paired loop region.

use crate::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx, WorkflowDigest};
use crate::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, WorkflowError, WorkflowParts, node,
};

fn n(
    id: u16,
    kind: CompiledNodeKind,
    out: Option<u16>,
    next: Option<u16>,
) -> crate::workflow::CompiledNode {
    node(
        StepIdx::new(id),
        kind,
        out.map(SlotIdx::new),
        next.map(StepIdx::new),
    )
}

fn build_concurrent_parts() -> WorkflowParts {
    let start = n(0, CompiledNodeKind::Nop, None, None);
    let lane = n(1, CompiledNodeKind::Nop, None, Some(2));
    let join = n(2, CompiledNodeKind::Nop, Some(0), Some(3));
    let finish = n(
        3,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    WorkflowParts {
        name: "ctor_barrier".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: Box::new([start, lane, join, finish]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([
            "start".into(),
            "lane".into(),
            "join".into(),
            "finish".into(),
        ]),
        input_slots: Box::new([]),
    }
}

#[test]
fn first_success_unpaired_lane_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_concurrent_parts();
    parts.nodes.get_mut(0).ok_or("fixture")?.kind = CompiledNodeKind::FirstSuccessStart {
        branches: Box::new([StepIdx::new(1)]),
        join: StepIdx::new(2),
    };
    let res = CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ());
    assert_eq!(
        res,
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(2)
        }),
        "expected exact LoopRegionUnpaired at join 2, got: {:?}",
        res
    );
    Ok(())
}

#[test]
fn foreach_unpaired_lane_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_concurrent_parts();
    parts.nodes.get_mut(0).ok_or("fixture")?.kind = CompiledNodeKind::ForEachConcurrentStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(0),
        limit: FanoutLimit::new(2),
        at_once: BranchCount::new(2),
        body: StepIdx::new(1),
        join: StepIdx::new(2),
    };
    let res = CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ());
    assert_eq!(
        res,
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(2)
        }),
        "expected exact LoopRegionUnpaired at join 2, got: {:?}",
        res
    );
    Ok(())
}

// ORIGINAL registered positive C4 cases restored using super::build_parts()

#[test]
fn first_success_ctor_is_still_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    // Valid paired FirstSuccess graph with original 4-node geometry from pair_tests.rs.
    let parts = super::build_parts();
    let mut modified = parts;
    modified.nodes.get_mut(0).ok_or("fixture")?.kind = CompiledNodeKind::FirstSuccessStart {
        branches: Box::new([StepIdx::new(1)]),
        join: StepIdx::new(2),
    };
    let parts_clone = modified.clone();
    let wf = CompiledWorkflow::try_from_concurrent_parts(modified)?;
    assert_eq!(
        wf.ir_profile(),
        crate::workflow::WorkflowIrProfile::ConcurrentV3
    );
    let reqs = wf.concurrent_requirements().ok_or("missing reqs")?;
    assert_eq!(reqs.frames(), 2);
    assert_eq!(reqs.groups(), 1);
    assert_eq!(wf.name(), "pair");
    let reconstructed = wf.to_parts();
    assert_eq!(reconstructed, parts_clone);
    Ok(())
}

fn restored_foreach_parts() -> Result<WorkflowParts, Box<dyn std::error::Error>> {
    let mut parts = super::build_parts();
    parts.nodes.get_mut(0).ok_or("fixture")?.kind = CompiledNodeKind::ForEachConcurrentStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(0),
        limit: FanoutLimit::new(2),
        at_once: BranchCount::new(2),
        body: StepIdx::new(1),
        join: StepIdx::new(2),
    };
    parts.input_slots = Box::new([CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::List,
        constraint: CompiledInputConstraint::required(),
    }]);
    Ok(parts)
}

#[test]
fn foreach_ctor_is_still_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    // Original paired geometry consumes the declared List input at slot 0.
    let parts = restored_foreach_parts()?;
    let parts_clone = parts.clone();
    let wf = CompiledWorkflow::try_from_concurrent_parts(parts)?;
    assert_eq!(
        wf.ir_profile(),
        crate::workflow::WorkflowIrProfile::ConcurrentV3
    );
    let reqs = wf.concurrent_requirements().ok_or("missing reqs")?;
    assert_eq!(reqs.frames(), 3);
    assert_eq!(reqs.groups(), 1);
    assert_eq!(wf.name(), "pair");
    let reconstructed = wf.to_parts();
    assert_eq!(reconstructed, parts_clone);
    Ok(())
}
