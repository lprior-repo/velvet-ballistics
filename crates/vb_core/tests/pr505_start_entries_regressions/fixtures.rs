use vb_core::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind as K,
    CompiledWorkflow, InputSlotKind, WorkflowError, WorkflowIrProfile, WorkflowParts, node,
};
use vb_core::{ResourceContract, WorkflowDigest};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
pub(super) enum Group {
    Together,
    FirstSuccess,
}

fn n(id: u16, kind: K, output: Option<u16>, next: Option<u16>) -> CompiledNode {
    node(StepIdx::new(id), kind, output.map(SlotIdx::new), next.map(StepIdx::new))
}

fn parts(nodes: Box<[CompiledNode]>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        name: "pr505-native-start-entries".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        step_names: Box::new([]),
        input_slots: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    }
}

pub(super) fn group(kind: Group, lanes: u16) -> TestResult<WorkflowParts> {
    let branches: Box<[StepIdx]> = match lanes {
        1 => Box::new([StepIdx::new(1)]),
        2 => Box::new([StepIdx::new(1), StepIdx::new(2)]),
        3 => Box::new([StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)]),
        _ => return Err("fixture supports one through three lanes".into()),
    };
    let join = lanes.checked_add(1).ok_or("fixture join overflow")?;
    let finish = join.checked_add(1).ok_or("fixture finish overflow")?;
    let count = usize::from(finish).checked_add(1).ok_or("fixture length overflow")?;
    let start = match kind {
        Group::Together => K::TogetherConcurrentStart { branches, join: StepIdx::new(join) },
        Group::FirstSuccess => K::FirstSuccessStart { branches, join: StepIdx::new(join) },
    };
    let mut nodes = Vec::new();
    nodes.try_reserve_exact(count)?;
    nodes.push(n(0, start, None, None));
    for id in 1..=lanes {
        nodes.push(n(id, K::ConcurrentLaneEnd { owner: StepIdx::new(0), result: None }, None, Some(join)));
    }
    nodes.push(n(join, K::ConcurrentGroupJoin { owner: StepIdx::new(0) }, Some(0), Some(finish)));
    nodes.push(n(finish, K::Finish { result: SlotIdx::new(0) }, None, None));
    let mut result = parts(nodes.into_boxed_slice(), 1);
    result.resource_contract.max_fanout = 3;
    Ok(result)
}

pub(super) fn foreach(input: u16, item: u16, limit: u32, at_once: u16) -> WorkflowParts {
    let nodes = Box::new([
        n(0, K::ForEachConcurrentStart {
            input: SlotIdx::new(input), item_slot: SlotIdx::new(item),
            limit: FanoutLimit::new(limit), at_once: BranchCount::new(at_once),
            body: StepIdx::new(1), join: StepIdx::new(2),
        }, None, None),
        n(1, K::ConcurrentLaneEnd { owner: StepIdx::new(0), result: None }, None, Some(2)),
        n(2, K::ConcurrentGroupJoin { owner: StepIdx::new(0) }, Some(2), Some(3)),
        n(3, K::Finish { result: SlotIdx::new(2) }, None, None),
    ]);
    let mut result = parts(nodes, 3);
    result.input_slots = Box::new([CompiledInputSlot {
        slot: SlotIdx::new(0), kind: InputSlotKind::List,
        constraint: CompiledInputConstraint::required(),
    }]);
    result.resource_contract.max_fanout = 2;
    // Keep default payload/step policy; declare the tested collection item bound.
    result.resource_contract.max_collect_items = 65_535;
    result
}

pub(super) fn node_mut(parts: &mut WorkflowParts, index: usize) -> TestResult<&mut CompiledNode> {
    parts.nodes.get_mut(index).ok_or_else(|| "missing fixture node".into())
}

pub(super) fn entries(parts: &mut WorkflowParts, values: &[u16]) -> TestResult {
    if values.len() > 3 { return Err("fixture entry bound".into()); }
    let mut converted = Vec::new();
    converted.try_reserve_exact(values.len())?;
    converted.extend(values.iter().copied().map(StepIdx::new));
    match &mut node_mut(parts, 0)?.kind {
        K::TogetherConcurrentStart { branches, .. } | K::FirstSuccessStart { branches, .. } => {
            *branches = converted.into_boxed_slice();
            Ok(())
        }
        _ => Err("not a group fixture".into()),
    }
}

pub(super) fn join(parts: &mut WorkflowParts, value: u16) -> TestResult {
    match &mut node_mut(parts, 0)?.kind {
        K::TogetherConcurrentStart { join, .. } | K::FirstSuccessStart { join, .. } => {
            *join = StepIdx::new(value);
            Ok(())
        }
        _ => Err("not a group fixture".into()),
    }
}

pub(super) fn body(parts: &mut WorkflowParts, value: u16) -> TestResult {
    match &mut node_mut(parts, 0)?.kind {
        K::ForEachConcurrentStart { body, .. } => {
            *body = StepIdx::new(value);
            Ok(())
        }
        _ => Err("not a foreach fixture".into()),
    }
}

pub(super) fn accepts(parts: WorkflowParts) -> TestResult {
    let workflow = CompiledWorkflow::try_from_concurrent_parts(parts)?;
    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert!(workflow.concurrent_requirements().is_some());
    Ok(())
}

pub(super) fn rejects(parts: WorkflowParts, expected: WorkflowError) {
    assert_eq!(CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()), Err(expected));
}

pub(super) fn crossing(to: u16) -> WorkflowError {
    WorkflowError::LoopRegionCrossing {
        control: StepIdx::new(0), from: StepIdx::new(0), to: StepIdx::new(to),
    }
}
