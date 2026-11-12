#![forbid(unsafe_code)]
//! Bounded ChooseSlot views; complete workflow fixtures exist only in native tests.
use super::ReplayError;
use crate::frame::{CheckpointStepState, RunFrameCheckpointV1};
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use crate::workflow::SlotBranch;
#[cfg(test)]
use crate::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, InputSlotKind, ResourceContract,
    WorkflowParts, node,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Shape {
    pub(super) two_branches: bool,
    pub(super) shifted_roots: bool,
    pub(super) reverse_roots: bool,
    pub(super) otherwise: bool,
}
impl Shape {
    pub(super) fn branch_count(self) -> u16 {
        if self.two_branches { 2 } else { 1 }
    }
    pub(super) fn root(self, second: bool) -> SlotIdx {
        SlotIdx::new(u16::from(self.shifted_roots) + u16::from(second != self.reverse_roots))
    }
    pub(super) fn slot_count(self) -> u16 {
        2 + u16::from(self.shifted_roots)
    }
    pub(super) fn node_count(self) -> u16 {
        2 + u16::from(self.two_branches) + u16::from(self.otherwise)
    }
    pub(super) fn otherwise_target(self) -> Option<StepIdx> {
        self.otherwise
            .then_some(StepIdx::new(self.branch_count() + 1))
    }
    pub(super) fn branches(self) -> Box<[SlotBranch]> {
        let first = SlotBranch {
            condition: self.root(false),
            target: StepIdx::new(1),
        };
        if self.two_branches {
            Box::new([
                first,
                SlotBranch {
                    condition: self.root(true),
                    target: StepIdx::new(2),
                },
            ])
        } else {
            Box::new([first])
        }
    }
    pub(super) fn checkpoint(self, a: bool, b: bool) -> RunFrameCheckpointV1 {
        let (first, second) = if self.reverse_roots { (b, a) } else { (a, b) };
        let first = Some(SlotValue::Bool(first));
        let second = Some(SlotValue::Bool(second));
        let slots = if self.shifted_roots {
            vec![None, first, second]
        } else {
            vec![first, second]
        };
        RunFrameCheckpointV1 {
            run_id: RunId::new(0),
            pc: StepIdx::ZERO,
            executed: 0,
            max_parallel_in_flight: u16::MAX,
            parallel_in_flight: 0,
            states: vec![CheckpointStepState::Pending; usize::from(self.node_count())],
            slots,
            taint: vec![Taint::Clean; usize::from(self.slot_count())],
        }
    }
    #[cfg(test)]
    pub(super) fn observe(parts: &WorkflowParts) -> Option<Self> {
        let CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } = &parts.nodes.first()?.kind
        else {
            return None;
        };
        if !(1..=2).contains(&branches.len()) || parts.input_slots.len() != 2 {
            return None;
        }
        let base = parts.input_slots.first()?.slot.get();
        let second = base.checked_add(1)?;
        if base > 1
            || parts.input_slots.get(1)?.slot.get() != second
            || parts.slot_count != second.checked_add(1)?
        {
            return None;
        }
        for input in &parts.input_slots {
            if input.kind != InputSlotKind::Bool || !input.constraint.requires_value() {
                return None;
            }
        }
        let first = branches.first()?.condition.get();
        if first != base && first != second {
            return None;
        }
        if branches.len() == 2
            && branches.get(1)?.condition.get() != if first == base { second } else { base }
        {
            return None;
        }
        Some(Self {
            two_branches: branches.len() == 2,
            shifted_roots: base == 1,
            reverse_roots: first != base,
            otherwise: otherwise.step().is_some(),
        })
    }
    pub(super) fn expected(self, a: bool, b: bool) -> Result<StepIdx, ReplayError> {
        let target = if a {
            Some(1)
        } else if self.two_branches && b {
            Some(2)
        } else if self.otherwise {
            Some(self.branch_count() + 1)
        } else {
            None
        };
        target.map(StepIdx::new).ok_or(ReplayError::Internal {
            reason: "choose_slot no branch matched and no otherwise",
        })
    }
}
impl Shape {
    #[cfg(test)]
    pub(super) fn parts(self) -> WorkflowParts {
        let branches = self.branches();
        let choose = node(
            StepIdx::ZERO,
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise: self.otherwise_target().into(),
            },
            None,
            None,
        );
        let terminal = |id| node(StepIdx::new(id), CompiledNodeKind::Nop, None, None);
        let nodes: Box<[crate::workflow::CompiledNode]> = match (self.two_branches, self.otherwise)
        {
            (false, false) => Box::new([choose, terminal(1)]),
            (true, true) => Box::new([choose, terminal(1), terminal(2), terminal(3)]),
            _ => Box::new([choose, terminal(1), terminal(2)]),
        };
        let input = |second| CompiledInputSlot {
            slot: SlotIdx::new(u16::from(self.shifted_roots) + u16::from(second)),
            kind: InputSlotKind::Bool,
            constraint: CompiledInputConstraint {
                presence: false.into(),
                ..CompiledInputConstraint::unconstrained()
            },
        };
        let slot_count = 2 + u16::from(self.shifted_roots);
        WorkflowParts {
            semantic_source_identity: None,
            source_metadata: None,
            name: "replay_choose".into(),
            digest: crate::ids::WorkflowDigest::from_bytes([0; 32]),
            nodes,
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract {
                max_steps: 2 + u16::from(self.two_branches) + u16::from(self.otherwise),
                max_slots: slot_count,
                max_constants: 0,
                max_accessors: 0,
                max_expressions: 0,
                max_expr_stack: 0,
                ..ResourceContract::DEFAULT
            },
            step_names: Box::new([]),
            input_slots: Box::new([input(false), input(true)]),
        }
    }
}
#[cfg(kani)]
impl kani::Arbitrary for Shape {
    fn any() -> Self {
        Self {
            two_branches: kani::any(),
            shifted_roots: kani::any(),
            reverse_roots: kani::any(),
            otherwise: kani::any(),
        }
    }
}

#[cfg(kani)]
pub(super) fn materialize(
    selected: Shape,
    mut check: impl FnMut(Shape, &[SlotBranch], Option<StepIdx>),
) {
    let branches = selected.branches();
    let otherwise = selected.otherwise_target();
    let observed = observe_view(&branches, otherwise, selected.slot_count());
    assert_eq!(observed, Some(selected));
    assert_eq!(observed.map(|actual| actual == selected), Some(true));
    assert_eq!(observed.map(|actual| selected == actual), Some(true));
    let mut matches = 0_u8;
    let mut checked = |shape, branches: &[SlotBranch], otherwise| {
        check(shape, branches, otherwise);
        matches += 1;
    };
    checked(selected, &branches, otherwise);
    assert_eq!(
        matches, 1,
        "symbolic shape must reach exactly one checked case"
    );
}

#[cfg(kani)]
fn observe_view(branches: &[SlotBranch], otherwise: Option<StepIdx>, slots: u16) -> Option<Shape> {
    if !(1..=2).contains(&branches.len()) || !(2..=3).contains(&slots) {
        return None;
    }
    let base = slots.checked_sub(2)?;
    let second = base.checked_add(1)?;
    let first = branches.first()?;
    if first.target != StepIdx::new(1)
        || (first.condition.get() != base && first.condition.get() != second)
    {
        return None;
    }
    if let Some(branch) = branches.get(1) {
        if branch.target != StepIdx::new(2)
            || branch.condition.get()
                != if first.condition.get() == base {
                    second
                } else {
                    base
                }
        {
            return None;
        }
    }
    let two_branches = branches.len() == 2;
    if otherwise.is_some_and(|target| target.get() != if two_branches { 3 } else { 2 }) {
        return None;
    }
    Some(Shape {
        two_branches,
        shifted_roots: base == 1,
        reverse_roots: first.condition.get() != base,
        otherwise: otherwise.is_some(),
    })
}
