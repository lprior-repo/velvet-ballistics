use vb_validate::ValidationError;
use vb_validate::control_flow::{
    StepFlow, WorkflowFlow, validate_control_flow, validate_forward_only_then,
};

fn flow(target: Option<usize>) -> WorkflowFlow {
    WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("entry".into()),
                branch_targets: vec![],
                then_target: Some(1),
            },
            StepFlow {
                id: Some("done".into()),
                branch_targets: vec![],
                then_target: target,
            },
        ],
    }
}

fn assert_rejected(target: usize, expected: ValidationError) {
    let flow = flow(Some(target));
    assert_eq!(validate_forward_only_then(&flow), Err(expected.clone()));
    assert_eq!(validate_control_flow(&flow), Err(expected));
}

#[test]
fn whole_validator_rejects_self_then() {
    assert_rejected(1, ValidationError::ControlFlowCycle);
}

#[test]
fn whole_validator_rejects_backward_then() {
    assert_rejected(0, ValidationError::ControlFlowCycle);
}

#[test]
fn whole_validator_rejects_then_at_step_count() {
    assert_rejected(2, ValidationError::InvalidThenTarget);
}

#[test]
fn whole_validator_rejects_maximum_then() {
    assert_rejected(usize::MAX, ValidationError::InvalidThenTarget);
}

#[test]
fn both_validators_accept_reachable_forward_then() {
    let flow = flow(None);
    assert_eq!(validate_forward_only_then(&flow), Ok(()));
    assert_eq!(validate_control_flow(&flow), Ok(()));
}

#[test]
fn numeric_structured_loop_keeps_its_legal_backedge() {
    use vb_core::ids::FanoutLimit;
    use vb_core::workflow::{CompiledNodeKind as K, ResourceContract, WorkflowParts, node};
    use vb_core::{SlotIdx, StepIdx, WorkflowDigest};
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: "structured_loop".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([
            node(
                StepIdx::new(0),
                K::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: FanoutLimit::new(4),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
            node(StepIdx::new(1), K::Nop, None, Some(StepIdx::new(2))),
            node(
                StepIdx::new(2),
                K::ForEachNext {
                    iterator_slot: SlotIdx::new(2),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
            node(
                StepIdx::new(3),
                K::ForEachJoin {
                    output: SlotIdx::new(0),
                },
                Some(SlotIdx::new(3)),
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    assert_eq!(
        vb_validate::gates::validate_gate_11_loop_body_graph(&parts),
        Ok(())
    );
}
