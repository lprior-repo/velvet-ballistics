#![forbid(unsafe_code)]

// Focus: symbolic DAG size/slots/targets/auxiliary nodes/metadata/taints;
// empty unused tables and the original default resource contract are projections.
use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{SlotValue, Taint};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, SlotBranch, WorkflowParts,
    node,
};

pub(super) struct ReplayShape {
    pub steps: u16,
    pub slots: u16,
    pub condition_a: SlotIdx,
    pub condition_b: SlotIdx,
    pub target_b: StepIdx,
    pub fallback: StepIdx,
    run_id: RunId,
    taint_a: Taint,
    taint_b: Taint,
}

impl kani::Arbitrary for ReplayShape {
    fn any() -> Self {
        let steps = bounded_u16(3, 8);
        let slots = bounded_u16(3, 8);
        let condition_a = bounded_slot(slots);
        let condition_b = bounded_slot(slots);
        kani::assume(condition_a != condition_b);
        Self {
            steps,
            slots,
            condition_a,
            condition_b,
            target_b: StepIdx::new(bounded_u16(1, steps.saturating_sub(1))),
            fallback: StepIdx::new(bounded_u16(1, steps.saturating_sub(1))),
            run_id: RunId::new(kani::any()),
            taint_a: kani::any(),
            taint_b: kani::any(),
        }
    }
}

fn bounded_u16(min: u16, max: u16) -> u16 {
    let value: u16 = kani::any();
    kani::assume(value >= min && value <= max);
    value
}

fn bounded_slot(count: u16) -> SlotIdx {
    SlotIdx::new(bounded_u16(0, count.saturating_sub(1)))
}

pub(super) fn two_branches(shape: &ReplayShape) -> Vec<SlotBranch> {
    vec![
        SlotBranch {
            condition: shape.condition_a,
            target: StepIdx::new(1),
        },
        SlotBranch {
            condition: shape.condition_b,
            target: shape.target_b,
        },
    ]
}

pub(super) fn one_branch(shape: &ReplayShape) -> Vec<SlotBranch> {
    vec![SlotBranch {
        condition: shape.condition_a,
        target: StepIdx::new(1),
    }]
}

fn generated_node(
    shape: &ReplayShape,
    branches: &[SlotBranch],
    fallback: Option<StepIdx>,
    id: u16,
) -> CompiledNode {
    let kind = if id == 0 {
        CompiledNodeKind::ChooseSlot {
            branches: branches.to_vec().into_boxed_slice(),
            otherwise: fallback.into(),
        }
    } else {
        auxiliary_kind(shape)
    };
    let output = kani::any::<bool>().then(|| bounded_slot(shape.slots));
    let next = generated_next(shape, branches, fallback, id);
    node(StepIdx::new(id), kind, output, next)
}

fn auxiliary_kind(shape: &ReplayShape) -> CompiledNodeKind {
    if kani::any() {
        CompiledNodeKind::Nop
    } else {
        CompiledNodeKind::Copy {
            source: bounded_slot(shape.slots),
        }
    }
}

fn generated_fixed<const N: usize>(
    shape: &ReplayShape,
    branches: &[SlotBranch],
    fallback: Option<StepIdx>,
) -> Box<[CompiledNode]> {
    let array: [CompiledNode; N] = std::array::from_fn(|i| {
        let id: u16 = match u16::try_from(i) {
            Ok(v) => v,
            Err(_) => {
                kani::assert(false, "array index out of u16 range");
                std::process::abort();
            }
        };
        generated_node(shape, branches, fallback, id)
    });
    Box::new(array)
}

fn parts_for<const N: usize>(
    shape: &ReplayShape,
    branches: &[SlotBranch],
    fallback: Option<StepIdx>,
) -> WorkflowParts {
    WorkflowParts {
        name: if kani::any() {
            "kani_plan"
        } else {
            "replay_shape"
        }
        .into(),
        digest: WorkflowDigest::from_bytes(kani::any()),
        semantic_source_identity: kani::any(),
        nodes: generated_fixed::<N>(shape, branches, fallback),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: shape.slots,
        symbols_count: kani::any(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    }
}

fn plan_for_size<const N: usize>(
    shape: &ReplayShape,
    branches: &[SlotBranch],
    fallback: Option<StepIdx>,
) -> Result<CompiledWorkflow, CoreError> {
    let parts = parts_for::<N>(shape, branches, fallback);
    CompiledWorkflow::try_from_parts(parts).map_err(|_| setup_error())
}

pub(super) fn plan(
    shape: &ReplayShape,
    branches: &[SlotBranch],
    fallback: Option<StepIdx>,
) -> Result<CompiledWorkflow, CoreError> {
    match shape.steps {
        3 => plan_for_size::<3>(shape, branches, fallback),
        4 => plan_for_size::<4>(shape, branches, fallback),
        5 => plan_for_size::<5>(shape, branches, fallback),
        6 => plan_for_size::<6>(shape, branches, fallback),
        7 => plan_for_size::<7>(shape, branches, fallback),
        8 => plan_for_size::<8>(shape, branches, fallback),
        _ => Err(setup_error()),
    }
}

pub(super) fn frame(shape: &ReplayShape, a: bool, b: bool) -> Result<RunFrame, CoreError> {
    let mut frame = RunFrame::new(shape.run_id, StepIdx::new(0), shape.steps, shape.slots)?;
    frame.write_slot_with_taint(shape.condition_a, SlotValue::Bool(a), shape.taint_a)?;
    frame.write_slot_with_taint(shape.condition_b, SlotValue::Bool(b), shape.taint_b)?;
    Ok(frame)
}

pub(super) fn setup_error() -> CoreError {
    CoreError::InvalidCompiledWorkflow {
        reason: "generated replay shape must be valid",
    }
}

pub(super) fn cover_shape(shape: &ReplayShape) {
    kani::cover(shape.steps == 3, "three nodes");
    kani::cover(shape.steps == 4, "four nodes");
    kani::cover(shape.steps == 5, "five nodes");
    kani::cover(shape.steps == 6, "six nodes");
    kani::cover(shape.steps == 7, "seven nodes");
    kani::cover(shape.steps == 8, "eight nodes");
    kani::cover(shape.slots == 3, "three slots");
    kani::cover(shape.slots == 4, "four slots");
    kani::cover(shape.slots == 5, "five slots");
    kani::cover(shape.slots == 6, "six slots");
    kani::cover(shape.slots == 7, "seven slots");
    kani::cover(shape.slots == 8, "eight slots");
    kani::cover(
        shape.condition_a == SlotIdx::new(0),
        "original first condition",
    );
    kani::cover(
        shape.condition_b == SlotIdx::new(1),
        "original second condition",
    );
}
fn generated_next(
    shape: &ReplayShape,
    branches: &[SlotBranch],
    fallback: Option<StepIdx>,
    id: u16,
) -> Option<StepIdx> {
    let next = id
        .checked_add(1)
        .filter(|next| *next < shape.steps)
        .map(StepIdx::new);
    if direct_successors(shape, branches, fallback) && kani::any::<bool>() {
        None
    } else {
        next
    }
}

fn direct_successors(
    shape: &ReplayShape,
    branches: &[SlotBranch],
    fallback: Option<StepIdx>,
) -> bool {
    let two = target_configured(branches, fallback, StepIdx::new(2));
    match shape.steps {
        3 => two,
        4 => two && target_configured(branches, fallback, StepIdx::new(3)),
        _ => false,
    }
}

fn target_configured(branches: &[SlotBranch], fallback: Option<StepIdx>, target: StepIdx) -> bool {
    branches.iter().any(|branch| branch.target == target) || fallback == Some(target)
}
