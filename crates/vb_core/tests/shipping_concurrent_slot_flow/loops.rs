use super::{assert_error, fixtures as f};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind as K, InputSlotKind, SlotFlowError, SlotFlowFailure,
    WorkflowParts, validate_slot_flow,
};

fn serial_foreach(result: u16) -> WorkflowParts {
    let group = group(6);
    let n1 = serial_start();
    let n2 = f::copy(2, 2, 3, 3);
    let n3 = serial_next();
    let n4 = f::n(4, K::Nop, None, Some(5));
    let n5 = f::end(5, 0, Some(result), 6);
    let n6 = f::join(6, 0, 4, 7);
    let n7 = f::finish(7, 4);
    let mut parts = f::parts(vec![group, n1, n2, n3, n4, n5, n6, n7], 5);
    f::declare(&mut parts, InputSlotKind::List);
    parts
}

fn repeat() -> WorkflowParts {
    let group = group(6);
    let n1 = repeat_start(4);
    let n2 = f::set(2, 0, 2, 3);
    let n3 = repeat_check(3, 4);
    let n4 = f::n(4, K::Nop, None, Some(5));
    let n5 = f::end(5, 0, Some(2), 6);
    let n6 = f::join(6, 0, 4, 7);
    let n7 = f::finish(7, 4);
    let mut parts = f::parts(vec![group, n1, n2, n3, n4, n5, n6, n7], 5);
    f::declare(&mut parts, InputSlotKind::I64);
    parts
}

fn repeat_first() -> WorkflowParts {
    let group = group(7);
    let n1 = repeat_start(5);
    let n2 = f::copy(2, 2, 3, 3);
    let n3 = f::set(3, 0, 2, 4);
    let n4 = repeat_check(4, 5);
    let n5 = f::n(5, K::Nop, None, Some(6));
    let n6 = f::end(6, 0, Some(2), 7);
    let n7 = f::join(7, 0, 4, 8);
    let n8 = f::finish(8, 4);
    let mut parts = f::parts(vec![group, n1, n2, n3, n4, n5, n6, n7, n8], 5);
    f::declare(&mut parts, InputSlotKind::I64);
    parts
}

#[test]
fn serial_foreach_iterator_definite() -> Result<(), SlotFlowError> {
    validate_slot_flow(&serial_foreach(1))
}
#[test]
fn serial_foreach_zero_path_cannot_initialize_body_result() {
    assert_error(
        serial_foreach(3),
        5,
        Some(3),
        SlotFlowFailure::UninitializedRead,
    );
}
#[test]
fn repeat_body_initializes_result() -> Result<(), SlotFlowError> {
    validate_slot_flow(&repeat())
}
#[test]
fn repeat_backedge_cannot_initialize_first_read() {
    assert_error(
        repeat_first(),
        2,
        Some(2),
        SlotFlowFailure::UninitializedRead,
    );
}

fn serial_start() -> CompiledNode {
    f::n(
        1,
        K::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(2),
            limit: 4u32.into(),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        },
        Some(1),
        None,
    )
}

fn serial_next() -> CompiledNode {
    f::n(
        3,
        K::ForEachNext {
            iterator_slot: SlotIdx::new(1),
            body: StepIdx::new(2),
            done: StepIdx::new(4),
        },
        Some(2),
        None,
    )
}

fn group(join: u16) -> CompiledNode {
    f::n(
        0,
        K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(join),
        },
        None,
        None,
    )
}

fn repeat_start(done: u16) -> CompiledNode {
    f::n(
        1,
        K::RepeatStart {
            max_attempts: 2u16.into(),
            body: StepIdx::new(2),
            done: StepIdx::new(done),
        },
        Some(1),
        None,
    )
}

fn repeat_check(id: u16, done: u16) -> CompiledNode {
    f::n(
        id,
        K::RepeatCheck {
            attempt_slot: SlotIdx::new(1),
            done: StepIdx::new(done),
        },
        None,
        Some(2),
    )
}
