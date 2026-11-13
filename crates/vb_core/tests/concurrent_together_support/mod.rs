mod base;

use vb_core::execution_lanes::GroupMode;
use vb_core::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind as K, WorkflowParts, node};

fn n(id: u16, kind: K, out: Option<u16>, next: Option<u16>) -> CompiledNode {
    node(
        StepIdx::new(id),
        kind,
        out.map(SlotIdx::new),
        next.map(StepIdx::new),
    )
}

fn group(mode: GroupMode, body: u16, join: u16) -> K {
    match mode {
        GroupMode::TogetherOrdered => K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(body)]),
            join: StepIdx::new(join),
        },
        GroupMode::FirstSuccess => K::FirstSuccessStart {
            branches: Box::new([StepIdx::new(body)]),
            join: StepIdx::new(join),
        },
        GroupMode::ForEachOrdered => K::ForEachConcurrentStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(0),
            limit: FanoutLimit::new(2),
            at_once: BranchCount::new(1),
            body: StepIdx::new(body),
            join: StepIdx::new(join),
        },
    }
}

fn end(id: u16, owner: u16, join: u16) -> CompiledNode {
    n(
        id,
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(owner),
            result: None,
        },
        None,
        Some(join),
    )
}

fn join(id: u16, owner: u16, out: u16, next: u16) -> CompiledNode {
    n(
        id,
        K::ConcurrentGroupJoin {
            owner: StepIdx::new(owner),
        },
        Some(out),
        Some(next),
    )
}

fn finish(id: u16) -> CompiledNode {
    n(
        id,
        K::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    )
}

pub(super) fn pair(outer: GroupMode, inner: GroupMode) -> WorkflowParts {
    let mut p = base::parts(K::Nop);
    p.nodes = Box::new([
        n(0, group(outer, 1, 5), None, None),
        n(1, group(inner, 2, 3), None, None),
        end(2, 1, 3),
        join(3, 1, 0, 4),
        end(4, 0, 5),
        join(5, 0, 1, 6),
        finish(6),
    ]);
    p
}

pub(super) fn deep(middle: GroupMode) -> WorkflowParts {
    let mut p = base::parts(K::Nop);
    p.nodes = Box::new([
        n(0, group(GroupMode::TogetherOrdered, 1, 8), None, None),
        n(1, group(middle, 2, 6), None, None),
        n(2, group(GroupMode::TogetherOrdered, 3, 4), None, None),
        end(3, 2, 4),
        join(4, 2, 0, 5),
        end(5, 1, 6),
        join(6, 1, 0, 7),
        end(7, 0, 8),
        join(8, 0, 1, 9),
        finish(9),
    ]);
    p
}
