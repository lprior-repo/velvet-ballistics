use vb_core::workflow::{ErrorSink, WorkflowParts, node, node_with_handler};
use vb_core::{
    CompiledNodeKind, CompiledWorkflow, ResourceContract, SlotIdx, StepIdx, WorkflowDigest,
    WorkflowError,
};

fn parts(
    inner: ErrorSink,
    outer: Option<SlotIdx>,
    body: StepIdx,
    handler: StepIdx,
) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: "error_handler_slot".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([
            node_with_handler(
                StepIdx::new(0),
                CompiledNodeKind::ErrorHandler {
                    body,
                    handler,
                    error_slot: inner,
                },
                None,
                None,
                StepIdx::new(2),
                outer,
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(2)),
            ),
            node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn normal(inner: ErrorSink, outer: Option<SlotIdx>) -> WorkflowParts {
    parts(inner, outer, StepIdx::new(1), StepIdx::new(2))
}

fn assert_both(parts: WorkflowParts, expected: Result<(), WorkflowError>) {
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts.clone()).map(|_| ()),
        expected
    );
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
        expected
    );
}

#[test]
fn checked_constructors_reject_internal_slot_at_count() {
    assert_both(
        normal(ErrorSink::Slot(SlotIdx::new(1)), Some(SlotIdx::new(0))),
        Err(WorkflowError::SlotOutOfBounds {
            slot: SlotIdx::new(1),
        }),
    );
}

#[test]
fn checked_constructors_reject_maximum_internal_slot() {
    assert_both(
        normal(ErrorSink::Slot(SlotIdx::new(u16::MAX)), None),
        Err(WorkflowError::SlotOutOfBounds {
            slot: SlotIdx::new(u16::MAX),
        }),
    );
}

#[test]
fn checked_constructors_accept_discard_and_bounded_internal_slot() {
    assert_both(normal(ErrorSink::Discard, None), Ok(()));
    assert_both(
        normal(ErrorSink::Slot(SlotIdx::new(0)), Some(SlotIdx::new(0))),
        Ok(()),
    );
}

#[test]
fn outer_error_slot_keeps_its_own_bounds_validation() {
    assert_both(
        normal(ErrorSink::Discard, Some(SlotIdx::new(1))),
        Err(WorkflowError::SlotOutOfBounds {
            slot: SlotIdx::new(1),
        }),
    );
}

#[test]
fn invalid_body_precedes_internal_slot_error() {
    assert_both(
        parts(
            ErrorSink::Slot(SlotIdx::new(1)),
            None,
            StepIdx::new(99),
            StepIdx::new(2),
        ),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99),
        }),
    );
}

#[test]
fn invalid_handler_precedes_internal_slot_error() {
    assert_both(
        parts(
            ErrorSink::Slot(SlotIdx::new(1)),
            None,
            StepIdx::new(1),
            StepIdx::new(99),
        ),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99),
        }),
    );
}
