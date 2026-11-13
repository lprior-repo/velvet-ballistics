#![forbid(unsafe_code)]
//! Checked execution artifact profile, independent of source-symbol carriers.

use super::{CompiledNodeKind, WorkflowError, WorkflowParts};

/// Workflow IR artifact profile.
///
/// This enum tags the execution-IR version produced by compilation. It is
/// distinct from source symbol versions (V1/V2) and YAML schema versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowIrProfile {
    /// Legacy sequential-only IR.
    LegacyV2,
    /// Concurrent-aware IR with parallel fan-out/fan-in nodes.
    ConcurrentV3,
}

/// Validate that a workflow's node graph is compatible with the given IR profile.
///
/// Returns `Ok(())` if compatible. If the workflow contains any concurrent
/// node types but the profile is `LegacyV2`, returns an error identifying
/// the first offending node.
pub(super) fn validate_profile(
    parts: &WorkflowParts,
    profile: WorkflowIrProfile,
) -> Result<(), WorkflowError> {
    if profile == WorkflowIrProfile::ConcurrentV3 {
        return Ok(());
    }

    let offender = parts.nodes.iter().find(|node| is_concurrent(&node.kind));
    if let Some(node) = offender {
        return Err(WorkflowError::ConcurrentProfileRequired { step: node.id });
    }

    Ok(())
}

fn is_concurrent(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ForEachConcurrentStart { .. }
            | CompiledNodeKind::TogetherConcurrentStart { .. }
            | CompiledNodeKind::FirstSuccessStart { .. }
            | CompiledNodeKind::ConcurrentLaneEnd { .. }
            | CompiledNodeKind::ConcurrentGroupJoin { .. }
    )
}
