use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind as K, InputSlotKind,
    ResourceContract, WorkflowParts,
};

pub(super) fn n(id: u16, kind: K, out: Option<u16>, next: Option<u16>) -> CompiledNode {
    vb_core::workflow::node(
        StepIdx::new(id),
        kind,
        out.map(SlotIdx::new),
        next.map(StepIdx::new),
    )
}

pub(super) fn end(id: u16, owner: u16, result: Option<u16>, join: u16) -> CompiledNode {
    n(
        id,
        K::ConcurrentLaneEnd {
            owner: StepIdx::new(owner),
            result: result.map(SlotIdx::new),
        },
        None,
        Some(join),
    )
}

pub(super) fn join(id: u16, owner: u16, out: u16, next: u16) -> CompiledNode {
    n(
        id,
        K::ConcurrentGroupJoin {
            owner: StepIdx::new(owner),
        },
        Some(out),
        Some(next),
    )
}

pub(super) fn finish(id: u16, result: u16) -> CompiledNode {
    n(
        id,
        K::Finish {
            result: SlotIdx::new(result),
        },
        None,
        None,
    )
}

pub(super) fn set(id: u16, value: u16, out: u16, next: u16) -> CompiledNode {
    n(
        id,
        K::SetConst {
            value: ConstIdx::new(value),
        },
        Some(out),
        Some(next),
    )
}

pub(super) fn copy(id: u16, source: u16, out: u16, next: u16) -> CompiledNode {
    n(
        id,
        K::Copy {
            source: SlotIdx::new(source),
        },
        Some(out),
        Some(next),
    )
}

pub(super) fn parts(nodes: Vec<CompiledNode>, slots: u16) -> WorkflowParts {
    WorkflowParts {
        name: "concurrent-flow".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(1), ConstValue::Bool(true)]),
        slot_count: slots,
        symbols_count: 1,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    }
}

pub(super) fn declare(p: &mut WorkflowParts, kind: InputSlotKind) {
    p.input_slots = Box::new([CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind,
        constraint: CompiledInputConstraint::required(),
    }]);
}

pub(super) fn foreach_owner(limit: u32) -> CompiledNode {
    n(
        0,
        K::ForEachConcurrentStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: limit.into(),
            at_once: 2u16.into(),
            body: StepIdx::new(1),
            join: StepIdx::new(3),
        },
        None,
        None,
    )
}

pub(super) fn foreach(limit: u32, parent_read: u16) -> WorkflowParts {
    let nodes = vec![
        foreach_owner(limit),
        copy(1, 1, 2, 2),
        end(2, 0, Some(2), 3),
        join(3, 0, 3, 4),
        finish(4, parent_read),
    ];
    let mut p = parts(nodes, 4);
    declare(&mut p, InputSlotKind::List);
    p
}

pub(super) fn fixed_owner(race: bool) -> CompiledNode {
    let kind = if race {
        K::FirstSuccessStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(3)]),
            join: StepIdx::new(5),
        }
    } else {
        K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(3)]),
            join: StepIdx::new(5),
        }
    };
    n(0, kind, None, None)
}

pub(super) fn fixed(race: bool, sibling_source: u16) -> WorkflowParts {
    let nodes = vec![
        fixed_owner(race),
        set(1, 0, 1, 2),
        end(2, 0, Some(1), 5),
        copy(3, sibling_source, 2, 4),
        end(4, 0, Some(2), 5),
        join(5, 0, 3, 6),
        finish(6, 3),
    ];
    let mut p = parts(nodes, 4);
    declare(&mut p, InputSlotKind::I64);
    p
}
