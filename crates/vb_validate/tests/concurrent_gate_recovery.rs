use vb_core::ids::{BranchCount, FanoutLimit};
use vb_core::workflow::{CompiledNodeKind as K, WorkflowError, WorkflowParts, node};
use vb_core::{ResourceContract, SlotIdx, StepIdx, WorkflowDigest};
use vb_validate::ValidationError;
use vb_validate::gates::{validate_gate_10_node_kind_specific, validate_gate_11_loop_body_graph};

fn parts(kind: K) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: "concurrent_gates".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([
            node(StepIdx::new(0), kind, None, None),
            node(
                StepIdx::new(1),
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(0),
                    result: None,
                },
                None,
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                K::ConcurrentGroupJoin {
                    owner: StepIdx::new(0),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(3)),
            ),
            node(
                StepIdx::new(3),
                K::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn foreach() -> K {
    K::ForEachConcurrentStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(1),
        limit: FanoutLimit::new(2),
        at_once: BranchCount::new(1),
        body: StepIdx::new(1),
        join: StepIdx::new(2),
    }
}

fn fixed(first_success: bool) -> K {
    let branches = Box::from([StepIdx::new(1)]);
    let join = StepIdx::new(2);
    if first_success {
        K::FirstSuccessStart { branches, join }
    } else {
        K::TogetherConcurrentStart { branches, join }
    }
}

fn rejected_node(kind: K, expected: WorkflowError) {
    assert_eq!(
        validate_gate_10_node_kind_specific(&parts(kind)),
        Err(ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: expected.to_string()
        })
    );
}

#[test]
fn gate_10_rejects_concurrent_foreach_bad_slot() {
    let mut kind = foreach();
    if let K::ForEachConcurrentStart { item_slot, .. } = &mut kind {
        *item_slot = SlotIdx::new(99);
    }
    rejected_node(
        kind,
        WorkflowError::SlotOutOfBounds {
            slot: SlotIdx::new(99),
        },
    );
}

#[test]
fn gate_10_rejects_together_bad_branch() {
    rejected_node(
        K::TogetherConcurrentStart {
            branches: Box::from([StepIdx::new(99)]),
            join: StepIdx::new(2),
        },
        WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99),
        },
    );
}

#[test]
fn gate_10_rejects_first_success_bad_join() {
    rejected_node(
        K::FirstSuccessStart {
            branches: Box::from([StepIdx::new(1)]),
            join: StepIdx::new(99),
        },
        WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99),
        },
    );
}

#[test]
fn gate_10_rejects_lane_end_bad_owner() {
    rejected_node(
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(99),
            result: None,
        },
        WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99),
        },
    );
}

#[test]
fn gate_10_rejects_group_join_bad_owner() {
    rejected_node(
        K::ConcurrentGroupJoin {
            owner: StepIdx::new(99),
        },
        WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99),
        },
    );
}

#[test]
fn gate_11_rejects_out_of_range_concurrent_body() {
    let mut kind = foreach();
    if let K::ForEachConcurrentStart { body, .. } = &mut kind {
        *body = StepIdx::new(99);
    }
    assert_eq!(
        validate_gate_11_loop_body_graph(&parts(kind)),
        Err(ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: WorkflowError::StepOutOfBounds {
                step: StepIdx::new(99)
            }
            .to_string()
        })
    );
}

#[test]
fn gate_11_rejects_join_owned_by_wrong_start() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = parts(fixed(false));
    parts.nodes.get_mut(2).ok_or("join")?.kind = K::ConcurrentGroupJoin {
        owner: StepIdx::new(1),
    };
    assert_eq!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: WorkflowError::LoopRegionUnpaired {
                control: StepIdx::new(2)
            }
            .to_string()
        })
    );
    Ok(())
}

#[test]
fn gate_11_rejects_missing_matching_join() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = parts(fixed(false));
    parts.nodes.get_mut(2).ok_or("join")?.kind = K::Nop;
    assert_eq!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: WorkflowError::LoopRegionUnpaired {
                control: StepIdx::new(2)
            }
            .to_string()
        })
    );
    Ok(())
}

#[test]
fn standalone_gates_accept_all_concurrent_modes() {
    for kind in [foreach(), fixed(false), fixed(true)] {
        let parts = parts(kind);
        assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }
}

#[test]
fn standalone_gates_keep_legacy_graphs_accepted() {
    let mut parts = parts(K::Nop);
    parts.nodes = Box::new([node(StepIdx::new(0), K::Nop, None, None)]);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
    assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
}
