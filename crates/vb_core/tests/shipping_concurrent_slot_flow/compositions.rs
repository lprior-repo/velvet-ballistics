use super::{assert_error, fixtures as f};
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind as K, InputSlotKind, SlotFlowError, SlotFlowFailure,
    WorkflowParts, validate_slot_flow,
};

fn s(n: u16) -> SlotIdx {
    SlotIdx::new(n)
}
fn t(n: u16) -> StepIdx {
    StepIdx::new(n)
}

fn collect_start(out: u16) -> CompiledNode {
    f::n(
        1,
        K::CollectStart {
            source: s(0),
            limit: 4u32.into(),
            page_size: 2,
            body: t(2),
            done: t(5),
        },
        Some(out),
        None,
    )
}
fn collect_page() -> CompiledNode {
    f::n(
        2,
        K::CollectPage {
            collector_slot: s(1),
            body: t(3),
            done: t(5),
        },
        None,
        None,
    )
}
fn collect_next() -> CompiledNode {
    f::n(
        4,
        K::CollectNext {
            collector_slot: s(1),
            body: t(2),
            done: t(5),
        },
        None,
        None,
    )
}
fn collect_finish(out: u16) -> CompiledNode {
    f::n(
        5,
        K::CollectFinish {
            collector_slot: s(1),
        },
        Some(out),
        Some(6),
    )
}

fn collect(result: u16) -> WorkflowParts {
    let nodes = vec![
        group(0, 1, 7, false),
        collect_start(1),
        collect_page(),
        f::copy(3, 1, 2, 4),
        collect_next(),
        collect_finish(3),
        f::end(6, 0, Some(result), 7),
        f::join(7, 0, 4, 8),
        f::finish(8, 4),
    ];
    let mut parts = f::parts(nodes, 5);
    f::declare(&mut parts, InputSlotKind::List);
    parts
}

#[test]
fn collect_finish_initialized() -> Result<(), SlotFlowError> {
    let parts = collect(3);
    validate_slot_flow(&parts)
}

#[test]
fn collect_page_exit_does_not_initialize_body_result() {
    let parts = collect(2);
    assert_error(parts, 6, Some(2), SlotFlowFailure::UninitializedRead);
}

fn reduce_start(out: u16) -> CompiledNode {
    f::n(
        1,
        K::ReduceStart {
            limit: 4,
            input: s(0),
            accumulator: s(2),
            initial: ConstIdx::new(0),
            body: t(2),
            done: t(6),
        },
        Some(out),
        None,
    )
}
fn reduce_next() -> CompiledNode {
    f::n(
        5,
        K::ReduceNext {
            iterator_slot: s(1),
            accumulator: s(2),
            body: t(2),
            done: t(6),
        },
        Some(0),
        None,
    )
}
fn reduce_finish(out: u16) -> CompiledNode {
    f::n(6, K::ReduceFinish { accumulator: s(2) }, Some(out), Some(7))
}

fn reduce() -> WorkflowParts {
    let nodes = vec![
        group(0, 1, 8, false),
        reduce_start(1),
        f::copy(2, 2, 3, 3),
        f::copy(3, 3, 4, 4),
        f::copy(4, 4, 2, 5),
        reduce_next(),
        reduce_finish(5),
        f::end(7, 0, Some(5), 8),
        f::join(8, 0, 6, 9),
        f::finish(9, 6),
    ];
    let mut parts = f::parts(nodes, 7);
    f::declare(&mut parts, InputSlotKind::List);
    parts
}

#[test]
fn reduce_carried_facts_are_reused() -> Result<(), SlotFlowError> {
    let parts = reduce();
    validate_slot_flow(&parts)
}

fn foreach_start(out: u16) -> CompiledNode {
    f::n(
        0,
        K::ForEachStart {
            input: s(0),
            item_slot: s(1),
            limit: 3u32.into(),
            body: t(1),
            done: t(6),
        },
        Some(out),
        None,
    )
}
fn foreach_next(out: u16) -> CompiledNode {
    f::n(
        5,
        K::ForEachNext {
            iterator_slot: s(2),
            body: t(1),
            done: t(6),
        },
        Some(out),
        None,
    )
}

fn around_foreach(result: u16) -> WorkflowParts {
    let nodes = vec![
        foreach_start(2),
        group(1, 2, 4, false),
        f::copy(2, 1, 3, 3),
        f::end(3, 1, Some(3), 4),
        f::join(4, 1, 4, 5),
        foreach_next(1),
        f::finish(6, result),
    ];
    let mut parts = f::parts(nodes, 5);
    f::declare(&mut parts, InputSlotKind::List);
    parts
}

#[test]
fn group_inside_serial_foreach_preserves_iterator() -> Result<(), SlotFlowError> {
    let parts = around_foreach(2);
    validate_slot_flow(&parts)
}

#[test]
fn zero_serial_foreach_does_not_initialize_group_output() {
    let parts = around_foreach(4);
    assert_error(parts, 6, Some(4), SlotFlowFailure::UninitializedRead);
}

fn repeat_start(out: u16) -> CompiledNode {
    f::n(
        0,
        K::RepeatStart {
            max_attempts: 2u16.into(),
            body: t(1),
            done: t(6),
        },
        Some(out),
        None,
    )
}
fn repeat_check() -> CompiledNode {
    f::n(
        5,
        K::RepeatCheck {
            attempt_slot: s(1),
            done: t(6),
        },
        None,
        Some(1),
    )
}

fn around_repeat() -> WorkflowParts {
    let nodes = vec![
        repeat_start(1),
        group(1, 2, 4, true),
        f::set(2, 0, 2, 3),
        f::end(3, 1, Some(2), 4),
        f::join(4, 1, 3, 5),
        repeat_check(),
        f::finish(6, 3),
    ];
    let parts = f::parts(nodes, 4);
    parts
}

#[test]
fn group_inside_repeat_reaches_result() -> Result<(), SlotFlowError> {
    let parts = around_repeat();
    validate_slot_flow(&parts)
}

fn group(id: u16, entry: u16, join: u16, race: bool) -> CompiledNode {
    let branches = Box::new([t(entry)]);
    let kind = if race {
        K::FirstSuccessStart {
            branches,
            join: t(join),
        }
    } else {
        K::TogetherConcurrentStart {
            branches,
            join: t(join),
        }
    };
    f::n(id, kind, None, None)
}
