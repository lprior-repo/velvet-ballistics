//! Basic taint source detection for compiled workflows.

use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

/// Identifies nodes that could introduce secret values into the workflow.
///
/// Currently detects `WaitEvent` and `Ask` nodes as potential secret sources,
/// since they receive external input that could contain sensitive data.
pub fn find_secret_sources(parts: &WorkflowParts) -> Vec<StepIdx> {
    let mut sources: Vec<StepIdx> = Vec::new();

    for node in parts.nodes.iter() {
        match node.kind {
            CompiledNodeKind::WaitEvent { .. } => {
                sources.push(node.id);
            }
            CompiledNodeKind::Ask { .. } => {
                sources.push(node.id);
            }
            _ => {}
        }
    }

    sources
}
