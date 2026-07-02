#![cfg(kani)]
#![forbid(unsafe_code)]

//! Focused, bounded Kani generators for workflow budget harnesses.

use crate::ids::{ActionId, SlotIdx, StepIdx};
use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};

pub(crate) struct BudgetWorkflowInputs {
    nodes: BudgetNodes,
    entry: StepIdx,
    contract: ResourceContract,
}

impl BudgetWorkflowInputs {
    pub(crate) fn nodes(&self) -> &[CompiledNode] {
        self.nodes.as_slice()
    }

    pub(crate) const fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) const fn entry(&self) -> StepIdx {
        self.entry
    }

    pub(crate) const fn contract(&self) -> &ResourceContract {
        &self.contract
    }

    pub(crate) fn is_focused_domain(&self) -> bool {
        let count = self.nodes.len();
        entry_is_valid(self.entry, count) && self.nodes.all_nodes_focused(count)
    }

    pub(crate) fn covers_nop(&self) -> bool {
        self.nodes.has_nop()
    }

    pub(crate) fn covers_do(&self) -> bool {
        self.nodes.has_do()
    }

    pub(crate) fn covers_wait_until(&self) -> bool {
        self.nodes.has_wait_until()
    }

    pub(crate) fn covers_finish(&self) -> bool {
        self.nodes.has_finish()
    }
}

enum BudgetNodes {
    Empty,
    One([CompiledNode; 1]),
    Two([CompiledNode; 2]),
}

impl BudgetNodes {
    const fn as_slice(&self) -> &[CompiledNode] {
        match self {
            Self::Empty => &[],
            Self::One(nodes) => nodes,
            Self::Two(nodes) => nodes,
        }
    }

    const fn len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::One(_) => 1,
            Self::Two(_) => 2,
        }
    }

    fn has_nop(&self) -> bool {
        self.has_kind(KindCover::Nop)
    }

    fn has_do(&self) -> bool {
        self.has_kind(KindCover::Do)
    }

    fn has_wait_until(&self) -> bool {
        self.has_kind(KindCover::WaitUntil)
    }

    fn has_finish(&self) -> bool {
        self.has_kind(KindCover::Finish)
    }

    fn has_kind(&self, cover: KindCover) -> bool {
        match self {
            Self::Empty => false,
            Self::One([node]) => kind_matches_cover(&node.kind, cover),
            Self::Two([first, second]) => {
                kind_matches_cover(&first.kind, cover) || kind_matches_cover(&second.kind, cover)
            }
        }
    }

    fn all_nodes_focused(&self, count: usize) -> bool {
        match self {
            Self::Empty => true,
            Self::One([node]) => node_is_focused(node, 0, count),
            Self::Two([first, second]) => {
                node_is_focused(first, 0, count) && node_is_focused(second, 1, count)
            }
        }
    }
}

#[derive(Clone, Copy)]
enum KindCover {
    Nop,
    Do,
    WaitUntil,
    Finish,
}

fn kind_matches_cover(kind: &CompiledNodeKind, cover: KindCover) -> bool {
    match cover {
        KindCover::Nop => matches!(kind, CompiledNodeKind::Nop),
        KindCover::Do => matches!(kind, CompiledNodeKind::Do { .. }),
        KindCover::WaitUntil => matches!(kind, CompiledNodeKind::WaitUntil { .. }),
        KindCover::Finish => matches!(kind, CompiledNodeKind::Finish { .. }),
    }
}

fn entry_is_valid(entry: StepIdx, node_count: usize) -> bool {
    node_count == 0 || step_is_valid(entry, node_count)
}

fn node_is_focused(node: &CompiledNode, position: u16, node_count: usize) -> bool {
    node.id == StepIdx::new(position)
        && maybe_step_is_valid(node.next, node_count)
        && maybe_step_is_valid(node.on_error, node_count)
        && kind_is_focused(&node.kind)
}

fn maybe_step_is_valid(step: Option<StepIdx>, node_count: usize) -> bool {
    match step {
        Some(value) => step_is_valid(value, node_count),
        None => true,
    }
}

fn step_is_valid(step: StepIdx, node_count: usize) -> bool {
    step.as_usize() < node_count
}

fn kind_is_focused(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::Nop
            | CompiledNodeKind::Do { .. }
            | CompiledNodeKind::WaitUntil { .. }
            | CompiledNodeKind::WaitEvent { .. }
            | CompiledNodeKind::Ask { .. }
            | CompiledNodeKind::Finish { .. }
    )
}

pub(crate) fn budget_workflow_inputs() -> BudgetWorkflowInputs {
    let nodes = budget_nodes();
    BudgetWorkflowInputs {
        entry: entry_for_nodes(&nodes),
        nodes,
        contract: kani::any::<ResourceContract>(),
    }
}

fn budget_nodes() -> BudgetNodes {
    let choice: u8 = kani::any();
    kani::assume(choice <= 2);
    match choice {
        0 => BudgetNodes::Empty,
        1 => BudgetNodes::One([budget_node(0, 1)]),
        _ => BudgetNodes::Two([budget_node(0, 2), budget_node(1, 2)]),
    }
}

fn entry_for_nodes(nodes: &BudgetNodes) -> StepIdx {
    match nodes.len() {
        0 => StepIdx::new(0),
        count => bounded_step_for_count(count),
    }
}

fn budget_node(position: u16, node_count: usize) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(position),
        output: maybe_slot(),
        next: maybe_step(node_count),
        on_error: maybe_step(node_count),
        error_slot: maybe_slot(),
        kind: budget_node_kind(),
    }
}

fn budget_node_kind() -> CompiledNodeKind {
    let choice: u8 = kani::any();
    kani::assume(choice <= 5);
    match choice {
        0 => CompiledNodeKind::Nop,
        1 => CompiledNodeKind::Do {
            action: ActionId::new(kani::any()),
            input: bounded_slot(),
        },
        2 => CompiledNodeKind::WaitUntil {
            deadline_slot: bounded_slot(),
        },
        3 => CompiledNodeKind::WaitEvent {
            event: bounded_slot(),
            timeout_slot: maybe_slot(),
        },
        4 => CompiledNodeKind::Ask {
            prompt: bounded_slot(),
            timeout_slot: maybe_slot(),
        },
        _ => CompiledNodeKind::Finish {
            result: bounded_slot(),
        },
    }
}

fn maybe_step(node_count: usize) -> Option<StepIdx> {
    if kani::any::<bool>() {
        Some(bounded_step_for_count(node_count))
    } else {
        None
    }
}

fn maybe_slot() -> Option<SlotIdx> {
    if kani::any::<bool>() {
        Some(bounded_slot())
    } else {
        None
    }
}

fn bounded_step_for_count(node_count: usize) -> StepIdx {
    match node_count {
        0 | 1 => StepIdx::new(0),
        _ => bounded_two_node_step(),
    }
}

fn bounded_two_node_step() -> StepIdx {
    if kani::any::<bool>() {
        StepIdx::new(1)
    } else {
        StepIdx::new(0)
    }
}

fn bounded_slot() -> SlotIdx {
    let value: u8 = kani::any();
    kani::assume(value <= 3);
    SlotIdx::new(u16::from(value))
}
