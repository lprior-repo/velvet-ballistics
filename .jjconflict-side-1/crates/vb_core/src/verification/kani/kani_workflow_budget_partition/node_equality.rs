#![forbid(unsafe_code)]

use crate::workflow::{CompiledNode, CompiledNodeKind};

// The original producer has at most two nodes of these six kinds. A true
// result implies full equality; other lengths and kinds return false.
pub(super) fn focused_nodes_equal(left: &[CompiledNode], right: &[CompiledNode]) -> bool {
    left.len() == right.len()
        && left.len() <= 2
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| node_equal(left, right))
}

fn node_equal(left: &CompiledNode, right: &CompiledNode) -> bool {
    let CompiledNode {
        id,
        output,
        routing,
        kind,
        retry,
    } = left;
    let CompiledNode {
        id: other_id,
        output: other_output,
        routing: other_routing,
        kind: other_kind,
        retry: other_retry,
    } = right;
    id == other_id
        && output == other_output
        && routing == other_routing
        && retry == other_retry
        && kind_equal(kind, other_kind)
}

fn kind_equal(left: &CompiledNodeKind, right: &CompiledNodeKind) -> bool {
    use CompiledNodeKind::{Ask, Do, Finish, Nop, WaitEvent, WaitUntil};
    match (left, right) {
        (Nop, Nop) => true,
        (
            Do { action, input },
            Do {
                action: other_action,
                input: other_input,
            },
        ) => action == other_action && input == other_input,
        (
            WaitUntil { deadline_slot },
            WaitUntil {
                deadline_slot: other_deadline,
            },
        ) => deadline_slot == other_deadline,
        (
            WaitEvent {
                event,
                timeout_slot,
            },
            WaitEvent {
                event: other_event,
                timeout_slot: other_timeout,
            },
        ) => event == other_event && timeout_slot == other_timeout,
        (
            Ask {
                prompt,
                timeout_slot,
            },
            Ask {
                prompt: other_prompt,
                timeout_slot: other_timeout,
            },
        ) => prompt == other_prompt && timeout_slot == other_timeout,
        (
            Finish { result },
            Finish {
                result: other_result,
            },
        ) => result == other_result,
        _ => false,
    }
}
