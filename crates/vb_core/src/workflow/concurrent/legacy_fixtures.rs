use super::*;
use crate::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind as K, InputSlotKind, node,
};

fn n(id: u16, kind: K, out: Option<u16>, next: Option<u16>) -> CompiledNode {
    node(
        StepIdx::new(id),
        kind,
        out.map(SlotIdx::new),
        next.map(StepIdx::new),
    )
}

fn inputs(p: &mut WorkflowParts) {
    p.slot_count = 5;
    p.input_slots = Box::new([CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::List,
        constraint: CompiledInputConstraint::required(),
    }]);
}

fn foreach_start_node(id: u16, body: u16, done: u16) -> CompiledNode {
    n(
        id,
        K::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(2),
            limit: 4.into(),
            body: StepIdx::new(body),
            done: StepIdx::new(done),
        },
        Some(1),
        None,
    )
}

fn foreach_next_node(id: u16, body: u16, done: u16) -> CompiledNode {
    n(
        id,
        K::ForEachNext {
            iterator_slot: SlotIdx::new(1),
            body: StepIdx::new(body),
            done: StepIdx::new(done),
        },
        Some(2),
        None,
    )
}

fn foreach_join_node(id: u16, output_slot: u16, next_id: u16) -> CompiledNode {
    n(
        id,
        K::ForEachJoin {
            output: SlotIdx::new(1),
        },
        Some(output_slot),
        Some(next_id),
    )
}

fn group_start_node(id: u16, branches: Box<[StepIdx]>, join: StepIdx) -> CompiledNode {
    n(
        id,
        K::TogetherConcurrentStart { branches, join },
        None,
        None,
    )
}

fn group_end_node(id: u16, owner: StepIdx, next_id: u16) -> CompiledNode {
    n(
        id,
        K::ConcurrentLaneEnd {
            owner,
            result: None,
        },
        None,
        Some(next_id),
    )
}

fn group_join_node(id: u16, owner: StepIdx, output_slot: u16, next_id: u16) -> CompiledNode {
    n(
        id,
        K::ConcurrentGroupJoin { owner },
        Some(output_slot),
        Some(next_id),
    )
}

fn finish_node(id: u16, result: SlotIdx) -> CompiledNode {
    n(id, K::Finish { result }, None, None)
}

fn copy_node(id: u16, source: SlotIdx, out_slot: u16, next_id: u16) -> CompiledNode {
    n(id, K::Copy { source }, Some(out_slot), Some(next_id))
}

pub(super) fn foreach_within_group() -> WorkflowParts {
    let mut p = build_parts();
    let b0 = Box::new([StepIdx::new(1)]);
    let nodes = Box::new([
        group_start_node(0, b0, StepIdx::new(6)),
        foreach_start_node(1, 2, 4),
        copy_node(2, SlotIdx::new(2), 3, 3),
        foreach_next_node(3, 2, 4),
        foreach_join_node(4, 3, 5),
        group_end_node(5, StepIdx::new(0), 6),
        group_join_node(6, StepIdx::new(0), 4, 7),
        finish_node(7, SlotIdx::new(4)),
    ]);
    p.nodes = nodes;
    inputs(&mut p);
    p
}

pub(super) fn group_within_foreach() -> WorkflowParts {
    let mut p = build_parts();
    let b1 = Box::new([StepIdx::new(2)]);
    let nodes = Box::new([
        foreach_start_node(0, 1, 5),
        group_start_node(1, b1, StepIdx::new(3)),
        group_end_node(2, StepIdx::new(1), 3),
        group_join_node(3, StepIdx::new(1), 3, 4),
        foreach_next_node(4, 1, 5),
        foreach_join_node(5, 4, 6),
        finish_node(6, SlotIdx::new(4)),
    ]);
    p.nodes = nodes;
    inputs(&mut p);
    p
}
