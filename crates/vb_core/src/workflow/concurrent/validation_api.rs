use super::{CompiledNodeKind, WorkflowError, WorkflowParts};

fn is_concurrent(kind: &CompiledNodeKind) -> bool {
    matches!(kind, super::concurrent_bootstrap::kinds!())
}

/// Validate one node's concurrent-specific numeric refs only.
pub fn validate_concurrent_node_kind(
    kind: &CompiledNodeKind,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    if !is_concurrent(kind) {
        return Ok(());
    }
    super::concurrent_kind_refs::validate(kind, parts)
}

/// Validate concurrent structure on actual geometry; new-kind-free graphs no-op.
pub fn validate_concurrent_structure(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    if !parts.nodes.iter().any(|node| is_concurrent(&node.kind)) {
        return Ok(());
    }
    super::concurrent_analyze::analyze(parts).map(drop)
}
