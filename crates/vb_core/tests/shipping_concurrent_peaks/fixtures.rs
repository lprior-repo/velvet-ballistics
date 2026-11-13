use crate::concurrent_regions::{ConcurrentGroupRegion, ConcurrentLaneRegion};
use crate::execution_lanes::GroupMode;
use crate::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx, WorkflowDigest};
use crate::workflow::{CompiledNode, CompiledNodeKind as K, ResourceContract, WorkflowParts, node};

fn contract() -> ResourceContract {
    let mut c = ResourceContract::DEFAULT;
    c.max_fanout = 256;
    c
}

fn n(id: u16, kind: K, next: Option<u16>) -> CompiledNode {
    node(StepIdx::new(id), kind, None, next.map(StepIdx::new))
}

fn start(cap: u16, limit: u32, body: u16, join: u16) -> K {
    K::ForEachConcurrentStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(1),
        limit: FanoutLimit::new(limit),
        at_once: BranchCount::new(cap),
        body: StepIdx::new(body),
        join: StepIdx::new(join),
    }
}

fn end(owner: u16) -> K {
    K::ConcurrentLaneEnd {
        owner: StepIdx::new(owner),
        result: None,
    }
}

fn join(owner: u16) -> K {
    K::ConcurrentGroupJoin {
        owner: StepIdx::new(owner),
    }
}

fn lane(entry: u16, exclusive: u16, terminal: u16, children: &[u16]) -> ConcurrentLaneRegion {
    let kids: Box<[StepIdx]> = children
        .iter()
        .copied()
        .map(StepIdx::new)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    ConcurrentLaneRegion {
        entry: StepIdx::new(entry),
        ends: Box::new([StepIdx::new(terminal)]),
        children: kids,
        exclusive_end: StepIdx::new(exclusive),
    }
}

fn group(
    owner: u16,
    join: u16,
    parent: Option<u16>,
    mode: GroupMode,
    capacity: u16,
    lanes: Vec<ConcurrentLaneRegion>,
) -> ConcurrentGroupRegion {
    ConcurrentGroupRegion {
        owner: StepIdx::new(owner),
        join: StepIdx::new(join),
        parent: parent.map(StepIdx::new),
        mode,
        capacity,
        lanes: lanes.into_boxed_slice(),
    }
}

fn foreach_group(
    owner: u16,
    join: u16,
    parent: Option<u16>,
    cap: u16,
    lane: ConcurrentLaneRegion,
) -> ConcurrentGroupRegion {
    group(
        owner,
        join,
        parent,
        GroupMode::ForEachOrdered,
        cap,
        vec![lane],
    )
}

fn parts(nodes: Vec<CompiledNode>) -> WorkflowParts {
    WorkflowParts {
        name: "test".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract(),
        step_names: Box::new([]),
        input_slots: Box::new([]),
    }
}

fn fixed_kind(first_success: bool, branches: &[u16], join: u16) -> K {
    let bs: Box<[StepIdx]> = branches
        .iter()
        .copied()
        .map(StepIdx::new)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    if first_success {
        K::FirstSuccessStart {
            branches: bs,
            join: StepIdx::new(join),
        }
    } else {
        K::TogetherConcurrentStart {
            branches: bs,
            join: StepIdx::new(join),
        }
    }
}

fn nested_nodes(outer: u16, inner: u16) -> Vec<CompiledNode> {
    let lim = u32::from(outer).max(20);
    let ilim = u32::from(inner).max(20);
    vec![
        n(0, start(outer, lim, 1, 5), None),
        n(1, start(inner, ilim, 2, 3), None),
        n(2, end(1), None),
        n(3, join(1), Some(4)),
        n(4, end(0), None),
        n(5, join(0), None),
    ]
}

pub(super) fn nested(outer: u16, inner: u16) -> (WorkflowParts, Vec<ConcurrentGroupRegion>) {
    let nodes = nested_nodes(outer, inner);
    let outer_g = foreach_group(0, 5, None, outer, lane(1, 5, 4, &[1]));
    let inner_g = foreach_group(1, 3, Some(0), inner, lane(2, 3, 2, &[]));
    (parts(nodes), vec![outer_g, inner_g])
}

pub(super) fn single(
    limit: u32,
    declared: u16,
    effective: u16,
) -> (WorkflowParts, Vec<ConcurrentGroupRegion>) {
    let nodes = vec![
        n(0, start(declared, limit, 1, 2), None),
        n(1, end(0), None),
        n(2, join(0), None),
    ];
    let g = foreach_group(0, 2, None, effective, lane(1, 2, 1, &[]));
    (parts(nodes), vec![g])
}

fn fixed_nodes(first_success: bool) -> Vec<CompiledNode> {
    vec![
        n(0, fixed_kind(first_success, &[1, 5], 9), None),
        n(1, start(2, 20, 2, 3), None),
        n(2, end(1), None),
        n(3, join(1), Some(4)),
        n(4, end(0), None),
        n(5, start(6, 20, 6, 7), None),
        n(6, end(5), None),
        n(7, join(5), Some(8)),
        n(8, end(0), None),
        n(9, join(0), None),
    ]
}

pub(super) fn fixed_nested(first_success: bool) -> (WorkflowParts, Vec<ConcurrentGroupRegion>) {
    let root_mode = if first_success {
        GroupMode::FirstSuccess
    } else {
        GroupMode::TogetherOrdered
    };
    let nodes = fixed_nodes(first_success);
    let root = group(
        0,
        9,
        None,
        root_mode,
        2,
        vec![lane(1, 5, 4, &[1]), lane(5, 9, 8, &[5])],
    );
    let left = foreach_group(1, 3, Some(0), 2, lane(2, 3, 2, &[]));
    let right = foreach_group(5, 7, Some(0), 6, lane(6, 7, 6, &[]));
    (parts(nodes), vec![root, left, right])
}

fn sequential_nodes() -> Vec<CompiledNode> {
    vec![
        n(0, fixed_kind(false, &[1, 2], 3), None),
        n(1, end(0), None),
        n(2, end(0), None),
        n(3, join(0), Some(4)),
        n(4, fixed_kind(false, &[5, 6, 7], 8), None),
        n(5, end(4), None),
        n(6, end(4), None),
        n(7, end(4), None),
        n(8, join(4), None),
    ]
}

pub(super) fn sequential() -> (WorkflowParts, Vec<ConcurrentGroupRegion>) {
    let nodes = sequential_nodes();
    let a = group(
        0,
        3,
        None,
        GroupMode::TogetherOrdered,
        2,
        vec![lane(1, 2, 1, &[]), lane(2, 3, 2, &[])],
    );
    let b = group(
        4,
        8,
        None,
        GroupMode::TogetherOrdered,
        3,
        vec![lane(5, 6, 5, &[]), lane(6, 7, 6, &[]), lane(7, 8, 7, &[])],
    );
    (parts(nodes), vec![a, b])
}
