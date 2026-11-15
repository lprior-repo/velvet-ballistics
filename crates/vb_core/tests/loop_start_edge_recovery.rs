#![forbid(unsafe_code)]

use vb_core::workflow::{CompiledInputConstraint, CompiledInputSlot, node};
use vb_core::{
    CompiledNodeKind as K, CompiledWorkflow, ConstIdx, ConstValue, InputSlotKind, ResourceContract,
    SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

fn parts(kind: K) -> WorkflowParts {
    WorkflowParts {
        name: "loop-start-edges".into(),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::List,
            constraint: CompiledInputConstraint::required(),
        }]),
        nodes: Box::new([
            node(
                StepIdx::new(0),
                kind,
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(1)),
            ),
            node(StepIdx::new(1), K::Nop, None, Some(StepIdx::new(2))),
            node(
                StepIdx::new(2),
                K::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]),
        constants: Box::new([ConstValue::I64(0)]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn check(make_kind: impl Fn(StepIdx) -> K) {
    let invalid = parts(make_kind(StepIdx::new(0)));
    let valid = parts(make_kind(StepIdx::new(1)));
    let expected = Err(WorkflowError::BackwardEdge {
        from: StepIdx::new(0),
        to: StepIdx::new(0),
    });
    assert_eq!(
        CompiledWorkflow::try_from_parts(invalid.clone()).map(|_| ()),
        expected
    );
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(invalid).map(|_| ()),
        expected
    );
    assert_eq!(
        CompiledWorkflow::try_from_parts(valid.clone()).map(|_| ()),
        Ok(())
    );
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(valid).map(|_| ()),
        Ok(())
    );
}

#[test]
fn collect_start_rejects_own_body_and_accepts_forward_body() {
    check(|body| K::CollectStart {
        source: SlotIdx::new(0),
        limit: 1.into(),
        page_size: 1,
        body,
        done: StepIdx::new(2),
    });
}

#[test]
fn reduce_start_rejects_own_body_and_accepts_forward_body() {
    check(|body| K::ReduceStart {
        limit: 1,
        input: SlotIdx::new(0),
        accumulator: SlotIdx::new(1),
        initial: ConstIdx::new(0),
        body,
        done: StepIdx::new(2),
    });
}

#[test]
fn repeat_start_rejects_own_body_and_accepts_forward_body() {
    check(|body| K::RepeatStart {
        max_attempts: 1.into(),
        body,
        done: StepIdx::new(2),
    });
}
