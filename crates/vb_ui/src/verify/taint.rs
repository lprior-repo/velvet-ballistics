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

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::ids::WorkflowDigest;
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts,
    };

    fn make_parts(kinds: Vec<CompiledNodeKind>) -> WorkflowParts {
        let nodes: Vec<CompiledNode> = kinds
            .into_iter()
            .enumerate()
            .map(|(i, kind)| CompiledNode {
                id: StepIdx::new(i as u16),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind,
            })
            .collect();
        let count = nodes.len();
        WorkflowParts {
            name: String::from("taint-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: (0..count).map(|_| Box::<str>::from("")).collect::<Vec<_>>().into_boxed_slice(),
        }
    }

    #[test]
    fn test_find_secret_sources_finds_wait_event_nodes() {
        let parts = make_parts(vec![
            CompiledNodeKind::Nop,
            CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let sources = find_secret_sources(&parts);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0], StepIdx::new(1));
    }

    #[test]
    fn test_find_secret_sources_finds_ask_nodes() {
        let parts = make_parts(vec![
            CompiledNodeKind::Ask {
                prompt: SlotIdx::new(1),
                timeout_slot: Some(SlotIdx::new(2)),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let sources = find_secret_sources(&parts);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0], StepIdx::new(0));
    }

    #[test]
    fn test_find_secret_sources_ignores_do_nodes() {
        use vb_core::ids::ActionId;
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let sources = find_secret_sources(&parts);
        assert!(sources.is_empty());
    }
}
