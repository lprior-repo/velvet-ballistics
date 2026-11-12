#![forbid(unsafe_code)]

use crate::workflow::{CompiledNode, CompiledNodeKind};

pub(super) fn index(nodes: &[CompiledNode]) -> Option<u8> {
    match nodes {
        [] => Some(0),
        [node] => tag(&node.kind)?.checked_add(1),
        [first, second] => tag(&first.kind)?
            .checked_mul(6)?
            .checked_add(tag(&second.kind)?)?
            .checked_add(7),
        _ => None,
    }
}

pub(super) fn matches(nodes: &[CompiledNode], expected: u8) -> bool {
    match nodes {
        [] => expected == 0,
        [node] => expected
            .checked_sub(1)
            .is_some_and(|wanted| kind_is(&node.kind, wanted)),
        [first, second] => {
            let Some(pair) = expected.checked_sub(7).filter(|pair| *pair < 36) else {
                return false;
            };
            match (pair.checked_div(6), pair.checked_rem(6)) {
                (Some(left), Some(right)) => {
                    kind_is(&first.kind, left) && kind_is(&second.kind, right)
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn tag(kind: &CompiledNodeKind) -> Option<u8> {
    match kind {
        CompiledNodeKind::Nop => Some(0),
        CompiledNodeKind::Do { .. } => Some(1),
        CompiledNodeKind::WaitUntil { .. } => Some(2),
        CompiledNodeKind::WaitEvent { .. } => Some(3),
        CompiledNodeKind::Ask { .. } => Some(4),
        CompiledNodeKind::Finish { .. } => Some(5),
        _ => None,
    }
}

fn kind_is(kind: &CompiledNodeKind, wanted: u8) -> bool {
    match wanted {
        0 => matches!(kind, CompiledNodeKind::Nop),
        1 => matches!(kind, CompiledNodeKind::Do { .. }),
        2 => matches!(kind, CompiledNodeKind::WaitUntil { .. }),
        3 => matches!(kind, CompiledNodeKind::WaitEvent { .. }),
        4 => matches!(kind, CompiledNodeKind::Ask { .. }),
        5 => matches!(kind, CompiledNodeKind::Finish { .. }),
        _ => false,
    }
}
