use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};

pub(super) fn with_nodes(
    nodes: &[CompiledNode],
    entry: StepIdx,
    contract: &ResourceContract,
    visit: impl FnOnce(&[CompiledNode], StepIdx, &ResourceContract),
) {
    match nodes {
        [] => visit(&[], entry, contract),
        [node] => with_focused_node(node.clone(), |node| visit(&[node], entry, contract)),
        [first, second] => with_focused_node(first.clone(), |first| {
            with_focused_node(second.clone(), |second| {
                visit(&[first, second], entry, contract)
            });
        }),
        _ => assert!(false, "legacy fixture supports at most two nodes"),
    }
}

fn with_focused_node(node: CompiledNode, visit: impl FnOnce(CompiledNode)) {
    match node.kind {
        CompiledNodeKind::Nop => visit(CompiledNode {
            kind: CompiledNodeKind::Nop,
            ..node
        }),
        CompiledNodeKind::Do { action, input } => visit(CompiledNode {
            kind: CompiledNodeKind::Do { action, input },
            ..node
        }),
        CompiledNodeKind::WaitUntil { deadline_slot } => visit(CompiledNode {
            kind: CompiledNodeKind::WaitUntil { deadline_slot },
            ..node
        }),
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => visit(CompiledNode {
            kind: CompiledNodeKind::WaitEvent {
                event,
                timeout_slot: timeout_slot.into(),
            },
            ..node
        }),
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => visit(CompiledNode {
            kind: CompiledNodeKind::Ask {
                prompt,
                timeout_slot: timeout_slot.into(),
            },
            ..node
        }),
        CompiledNodeKind::Finish { result } => visit(CompiledNode {
            kind: CompiledNodeKind::Finish { result },
            ..node
        }),
        _ => assert!(false, "budget generator produced an unsupported node kind"),
    }
}
