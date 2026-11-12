use super::{BudgetNodes, BudgetWorkflowInputs};
use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind};

pub(crate) fn focused_slice_domain(nodes: &[CompiledNode], entry: StepIdx) -> bool {
    match nodes {
        [] => true,
        [first] => entry.as_usize() < 1 && slice_node_focused(first, 0, 1),
        [first, second] => {
            entry.as_usize() < 2
                && slice_node_focused(first, 0, 2)
                && slice_node_focused(second, 1, 2)
        }
        _ => false,
    }
}

fn slice_node_focused(node: &CompiledNode, position: u16, count: usize) -> bool {
    node.id == StepIdx::new(position)
        && slice_step_valid(node.routing.next.step(), count)
        && slice_step_valid(node.routing.error.handler(), count)
        && matches!(
            node.kind,
            CompiledNodeKind::Nop
                | CompiledNodeKind::Do { .. }
                | CompiledNodeKind::WaitUntil { .. }
                | CompiledNodeKind::WaitEvent { .. }
                | CompiledNodeKind::Ask { .. }
                | CompiledNodeKind::Finish { .. }
        )
}

fn slice_step_valid(step: Option<StepIdx>, count: usize) -> bool {
    match step {
        None => true,
        Some(step) => step.as_usize() < count,
    }
}

#[kani::proof]
#[kani::unwind(6)]
fn focused_domain_enum_and_slice_are_equivalent() {
    let nodes = match kani::any::<u8>() {
        0 => BudgetNodes::Empty,
        1 => BudgetNodes::One([kani::any::<CompiledNode>()]),
        _ => BudgetNodes::Two([kani::any::<CompiledNode>(), kani::any::<CompiledNode>()]),
    };
    let inputs = BudgetWorkflowInputs {
        nodes,
        entry: StepIdx::new(kani::any()),
        contract: kani::any(),
    };
    let old = inputs.is_focused_domain();
    let nodes = inputs.nodes();
    let borrowed = focused_slice_domain(nodes, inputs.entry());
    assert_eq!(old, borrowed, "enum and borrowed-slice domains must agree");

    kani::cover!(nodes.is_empty(), "equivalence includes zero nodes");
    kani::cover!(nodes.len() == 1, "equivalence includes one node");
    kani::cover!(nodes.len() == 2, "equivalence includes two nodes");
    kani::cover!(
        nodes.is_empty() && inputs.entry() == StepIdx::new(u16::MAX) && old,
        "empty input keeps the original unrestricted-entry domain"
    );
    kani::cover!(nodes.len() == 1 && old, "one-node acceptance is reachable");
    kani::cover!(nodes.len() == 1 && !old, "one-node rejection is reachable");
    kani::cover!(nodes.len() == 2 && old, "two-node acceptance is reachable");
    kani::cover!(nodes.len() == 2 && !old, "two-node rejection is reachable");
    kani::cover!(old && inputs.covers_nop(), "accepted domain includes Nop");
    kani::cover!(old && inputs.covers_do(), "accepted domain includes Do");
    kani::cover!(
        old && inputs.covers_wait_until(),
        "accepted domain includes WaitUntil"
    );
    kani::cover!(
        old && nodes
            .iter()
            .any(|node| matches!(node.kind, CompiledNodeKind::WaitEvent { .. })),
        "accepted domain includes WaitEvent"
    );
    kani::cover!(
        old && nodes
            .iter()
            .any(|node| matches!(node.kind, CompiledNodeKind::Ask { .. })),
        "accepted domain includes Ask"
    );
    kani::cover!(
        old && inputs.covers_finish(),
        "accepted domain includes Finish"
    );
    kani::cover!(
        old && nodes
            .iter()
            .any(|node| node.routing.error.handler().is_some()),
        "valid error edges remain accepted"
    );
    kani::cover!(
        old && matches!(nodes, [first, second]
            if first.routing.error.handler() == Some(StepIdx::new(1))
                && second.routing.error.handler() == Some(StepIdx::new(0))),
        "two-node error cycles remain accepted"
    );
    kani::cover!(
        !nodes.is_empty() && inputs.entry().as_usize() >= nodes.len() && !old,
        "invalid entry rejection is reachable"
    );
    kani::cover!(
        !old && nodes
            .iter()
            .enumerate()
            .any(|(index, node)| node.id.as_usize() != index),
        "invalid node identity rejection is reachable"
    );
    kani::cover!(
        !old && nodes.iter().any(|node| {
            node.routing
                .next
                .step()
                .is_some_and(|step| step.as_usize() >= nodes.len())
        }),
        "invalid normal edge rejection is reachable"
    );
    kani::cover!(
        !old && nodes.iter().any(|node| {
            node.routing
                .error
                .handler()
                .is_some_and(|step| step.as_usize() >= nodes.len())
        }),
        "invalid error edge rejection is reachable"
    );
    kani::cover!(
        !old && nodes
            .iter()
            .any(|node| matches!(node.kind, CompiledNodeKind::SetConst { .. })),
        "unsupported kind rejection is reachable"
    );
}
