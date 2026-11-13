#![forbid(unsafe_code)]

use vb_core::{CompiledNode, CompiledNodeKind as K};

pub(super) fn supports(node: &CompiledNode) -> bool {
    let handler = node.routing.error.handler().is_some();
    let error_slot = node.routing.error.error_slot().is_some();
    (!handler && !error_slot) || (handler && matches!(node.kind, K::Do { .. } | K::EvalExpr { .. }))
}
