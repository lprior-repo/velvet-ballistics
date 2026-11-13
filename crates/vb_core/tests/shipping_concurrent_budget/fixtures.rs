use super::{K, n};
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::workflow::CompiledNode;

pub(super) fn foreach(id: u16, limit: u32, cap: u16, targets: (u16, u16)) -> CompiledNode {
    n(
        id,
        K::ForEachConcurrentStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: limit.into(),
            at_once: cap.into(),
            body: StepIdx::new(targets.0),
            join: StepIdx::new(targets.1),
        },
        None,
    )
}

pub(super) fn action(id: u16, next: u16) -> CompiledNode {
    n(
        id,
        K::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
        Some(next),
    )
}
pub(super) fn end(id: u16, owner: u16, next: u16) -> CompiledNode {
    n(
        id,
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(owner),
            result: None,
        },
        Some(next),
    )
}
pub(super) fn join(id: u16, owner: u16, next: u16) -> CompiledNode {
    n(
        id,
        K::ConcurrentGroupJoin {
            owner: StepIdx::new(owner),
        },
        Some(next),
    )
}
fn inner_body() -> Vec<CompiledNode> {
    vec![action(2, 3), end(3, 1, 4), join(4, 1, 5)]
}

fn outer_tail() -> Vec<CompiledNode> {
    vec![
        n(
            5,
            K::ConcurrentLaneEnd {
                owner: StepIdx::new(0),
                result: None,
            },
            Some(6),
        ),
        n(
            6,
            K::ConcurrentGroupJoin {
                owner: StepIdx::new(0),
            },
            Some(7),
        ),
        n(
            7,
            K::Finish {
                result: SlotIdx::new(0),
            },
            None,
        ),
    ]
}

pub(super) fn nested(outer: u32, outer_cap: u16, inner: u32, inner_cap: u16) -> Vec<CompiledNode> {
    let mut nodes = vec![
        foreach(0, outer, outer_cap, (1, 6)),
        foreach(1, inner, inner_cap, (2, 4)),
    ];
    nodes.extend(inner_body());
    nodes.extend(outer_tail());
    nodes
}

pub(super) fn finish(id: u16) -> CompiledNode {
    n(
        id,
        K::Finish {
            result: SlotIdx::new(0),
        },
        None,
    )
}
