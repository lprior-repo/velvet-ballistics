use super::validate_concurrent_pair;
use crate::ids::{SlotIdx, StepIdx, WorkflowDigest};
use crate::workflow::{CompiledNode, WorkflowParts, node};
use crate::workflow::{CompiledNodeKind as K, ResourceContract, WorkflowError};

fn pair_start() -> CompiledNode {
    node(
        StepIdx::new(0),
        K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(2),
        },
        None,
        None,
    )
}

fn pair_lane() -> CompiledNode {
    node(
        StepIdx::new(1),
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        },
        None,
        Some(StepIdx::new(2)),
    )
}

fn pair_join() -> CompiledNode {
    node(
        StepIdx::new(2),
        K::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(3)),
    )
}

fn pair_finish() -> CompiledNode {
    node(
        StepIdx::new(3),
        K::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )
}

fn build_nodes() -> Box<[CompiledNode]> {
    Box::new([pair_start(), pair_lane(), pair_join(), pair_finish()])
}

fn build_parts() -> WorkflowParts {
    WorkflowParts {
        name: "pair".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: build_nodes(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        step_names: Box::new([]),
        input_slots: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    }
}

#[test]
fn matching_pair_validates() -> Result<(), Box<dyn std::error::Error>> {
    let parts = build_parts();
    validate_concurrent_pair(&parts, StepIdx::new(0))?;
    Ok(())
}

#[test]
fn join_index_at_len_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let n = parts.nodes.get_mut(0).ok_or("missing fixture node")?;
    n.kind = K::TogetherConcurrentStart {
        branches: Box::new([StepIdx::new(1)]),
        join: StepIdx::new(4),
    };
    assert_eq!(
        validate_concurrent_pair(&parts, StepIdx::new(0)),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(4)
        })
    );
    Ok(())
}

#[test]
fn join_before_start_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let n = parts.nodes.get_mut(0).ok_or("missing fixture node")?;
    n.kind = K::TogetherConcurrentStart {
        branches: Box::new([StepIdx::new(1)]),
        join: StepIdx::new(0),
    };
    assert_eq!(
        validate_concurrent_pair(&parts, StepIdx::new(0)),
        Err(WorkflowError::BackwardEdge {
            from: StepIdx::new(0),
            to: StepIdx::new(0)
        })
    );
    Ok(())
}

#[test]
fn join_missing_owner_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let n = parts.nodes.get_mut(2).ok_or("missing fixture node")?;
    n.kind = K::ConcurrentGroupJoin {
        owner: StepIdx::new(1),
    };
    assert_eq!(
        validate_concurrent_pair(&parts, StepIdx::new(0)),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(2)
        })
    );
    Ok(())
}

#[test]
fn join_next_at_len_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let n = parts.nodes.get_mut(2).ok_or("missing fixture node")?;
    n.routing.next = Some(StepIdx::new(4)).into();
    assert_eq!(
        validate_concurrent_pair(&parts, StepIdx::new(0)),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(4)
        })
    );
    Ok(())
}

#[test]
fn join_next_before_start_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    let n = parts.nodes.get_mut(2).ok_or("missing fixture node")?;
    n.routing.next = Some(StepIdx::new(1)).into();
    assert_eq!(
        validate_concurrent_pair(&parts, StepIdx::new(0)),
        Err(WorkflowError::BackwardEdge {
            from: StepIdx::new(2),
            to: StepIdx::new(1)
        })
    );
    Ok(())
}
fn backward_to(to: u16) -> WorkflowError {
    WorkflowError::BackwardEdge {
        from: StepIdx::new(2),
        to: StepIdx::new(to),
    }
}

fn continuation_cases() -> [(Option<u16>, WorkflowError); 4] {
    [
        (
            None,
            WorkflowError::LoopRegionUnpaired {
                control: StepIdx::new(2),
            },
        ),
        (Some(1), backward_to(1)),
        (Some(2), backward_to(2)),
        (
            Some(4),
            WorkflowError::StepOutOfBounds {
                step: StepIdx::new(4),
            },
        ),
    ]
}

#[test]
fn pair_requires_forward_inbounds_continuation() -> Result<(), Box<dyn std::error::Error>> {
    for (next, expected) in continuation_cases() {
        let mut parts = build_parts();
        let join = parts.nodes.get_mut(2).ok_or("missing join")?;
        join.routing.next = next.map(StepIdx::new).into();
        assert_eq!(
            validate_concurrent_pair(&parts, StepIdx::new(0)),
            Err(expected)
        );
    }
    Ok(())
}

#[path = "pair_constructor_tests.rs"]
mod constructors;

#[path = "lane_end_tests.rs"]
mod lane_ends;

#[path = "start_entries_tests.rs"]
mod start_entries;

#[path = "targets_tests.rs"]
mod targets;

#[path = "lane_walk_tests.rs"]
mod lane_walk;

#[path = "ownership_tests.rs"]
mod ownership;

#[path = "legacy_fixtures.rs"]
mod legacy_fixtures;

#[path = "legacy_controls_tests.rs"]
mod legacy_controls;

#[path = "cursor_tests.rs"]
mod cursor_controls;

#[path = "repeat_compositions_tests.rs"]
mod repeat_compositions;

#[path = "collect_compositions_tests.rs"]
mod collect_compositions;

#[path = "reduce_compositions_tests.rs"]
mod reduce_compositions;

#[path = "normal_exit_tests.rs"]
mod normal_exit_controls;
