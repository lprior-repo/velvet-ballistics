//! Taint flow overlay -- traces secret-to-sink paths through the workflow graph.
//!
//! Given a WorkflowParts, identifies:
//! - Secret source nodes (WaitEvent, Ask)
//! - Reachable sink nodes (Finish)
//! - Paths from source to sink
//! - Whether the Finish node is safe (no secret reached it)

use std::collections::HashSet;
use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

/// Severity of a taint path segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaintPathStatus {
    /// Path from source to sink does not reach a Finish node.
    Warning,
    /// Path from source reaches a Finish node -- secret may leak.
    Dangerous,
}

/// One edge in a taint propagation path.
#[derive(Debug, Clone)]
pub struct TaintPathSegment {
    /// Source step of this edge.
    pub from: StepIdx,
    /// Destination step of this edge.
    pub to: StepIdx,
    /// Whether this edge is part of a dangerous path to Finish.
    pub status: TaintPathStatus,
}

/// Complete taint overlay result for a workflow.
#[derive(Debug, Clone)]
pub struct TaintOverlayResult {
    /// All secret source step indices.
    pub sources: Vec<StepIdx>,
    /// All sink (Finish) step indices.
    pub sinks: Vec<StepIdx>,
    /// Path segments coloured by severity.
    pub paths: Vec<TaintPathSegment>,
    /// True when no source can reach any sink.
    pub finish_safe: bool,
}

/// Compute the full taint overlay for a compiled workflow.
///
/// 1. Identify secret sources (WaitEvent, Ask).
/// 2. Identify sinks (Finish).
/// 3. BFS forward from each source through `next` edges.
/// 4. If a source reaches a Finish, mark the path as Dangerous.
/// 5. If a source reaches other nodes but not Finish, mark as Warning.
/// 6. `finish_safe` is true when no source can reach any sink.
pub fn compute_taint_overlay(parts: &WorkflowParts) -> TaintOverlayResult {
    let sources = find_sources(parts);
    let sinks = find_sinks(parts);

    let sink_set: HashSet<StepIdx> = sinks.iter().copied().collect();
    let mut paths: Vec<TaintPathSegment> = Vec::new();
    let mut any_source_reaches_sink = false;

    for source in &sources {
        let reachable = walk_forward(parts, *source);
        let reachable_set: HashSet<StepIdx> = reachable.iter().copied().collect();
        let reaches_sink = reachable_set.intersection(&sink_set).count() > 0;

        if reaches_sink {
            any_source_reaches_sink = true;
        }

        let status = if reaches_sink {
            TaintPathStatus::Dangerous
        } else {
            TaintPathStatus::Warning
        };

        // Emit path segments from source to each reachable node.
        for step in &reachable {
            paths.push(TaintPathSegment {
                from: *source,
                to: *step,
                status,
            });
        }
    }

    TaintOverlayResult {
        sources,
        sinks,
        paths,
        finish_safe: !any_source_reaches_sink,
    }
}

/// BFS forward from `start`, following `next` edges only.
/// Returns all reachable step indices (excluding `start` itself).
fn walk_forward(parts: &WorkflowParts, start: StepIdx) -> Vec<StepIdx> {
    let node_count = parts.nodes.len();
    let mut visited = HashSet::new();
    visited.insert(start);
    let mut result = Vec::new();
    let mut queue = Vec::new();

    // Seed with the successors of `start`.
    if let Some(node) = parts.nodes.get(start.as_usize()) {
        enqueue_successors(node, node_count, &mut visited, &mut queue);
    }

    while let Some(current) = queue.pop() {
        result.push(current);

        if let Some(node) = parts.nodes.get(current.as_usize()) {
            enqueue_successors(node, node_count, &mut visited, &mut queue);
        }
    }

    result
}

/// Enqueue the linear successor(s) of a node for BFS traversal.
fn enqueue_successors(
    node: &vb_core::workflow::CompiledNode,
    node_count: usize,
    visited: &mut HashSet<StepIdx>,
    queue: &mut Vec<StepIdx>,
) {
    // Follow `next` edge only for the overlay model.
    if let Some(next) = node.next {
        let next_usize = next.as_usize();
        if next_usize < node_count && visited.insert(next) {
            queue.push(next);
        }
    }
}

/// Collect WaitEvent and Ask nodes as secret sources.
fn find_sources(parts: &WorkflowParts) -> Vec<StepIdx> {
    let mut sources = Vec::new();
    for node in parts.nodes.iter() {
        match node.kind {
            CompiledNodeKind::WaitEvent { .. } | CompiledNodeKind::Ask { .. } => {
                sources.push(node.id);
            }
            _ => {}
        }
    }
    sources
}

/// Collect Finish nodes as sinks.
fn find_sinks(parts: &WorkflowParts) -> Vec<StepIdx> {
    let mut sinks = Vec::new();
    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Finish { .. } = node.kind {
            sinks.push(node.id);
        }
    }
    sinks
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{SlotIdx, WorkflowDigest};
    use vb_core::workflow::{CompiledNode, ResourceContract};

    fn make_parts_with_next(
        kinds: Vec<(CompiledNodeKind, Option<StepIdx>)>,
    ) -> WorkflowParts {
        let nodes: Vec<CompiledNode> = kinds
            .into_iter()
            .enumerate()
            .map(|(i, (kind, next))| CompiledNode {
                id: StepIdx::new(i as u16),
                output: None,
                next,
                on_error: None,
                error_slot: None,
                kind,
            })
            .collect();
        let count = nodes.len();
        WorkflowParts {
            name: String::from("overlay-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: (0..count)
                .map(|_| Box::<str>::from(""))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    #[test]
    fn test_no_sources_no_sinks() {
        let parts = make_parts_with_next(vec![
            (CompiledNodeKind::Nop, Some(StepIdx::new(1))),
            (CompiledNodeKind::Nop, None),
        ]);
        let result = compute_taint_overlay(&parts);
        assert!(result.sources.is_empty());
        assert!(result.sinks.is_empty());
        assert!(result.paths.is_empty());
        assert!(result.finish_safe);
    }

    #[test]
    fn test_source_cannot_reach_sink() {
        // WaitEvent at 0 -> Nop at 1 (no next, no Finish)
        let parts = make_parts_with_next(vec![
            (
                CompiledNodeKind::WaitEvent {
                    event: SlotIdx::new(0),
                    timeout_slot: None,
                },
                Some(StepIdx::new(1)),
            ),
            (CompiledNodeKind::Nop, None),
        ]);
        let result = compute_taint_overlay(&parts);
        assert_eq!(result.sources.len(), 1);
        assert_eq!(result.sources[0], StepIdx::new(0));
        assert!(result.sinks.is_empty());
        assert!(result.finish_safe);
        // Should have Warning segments (source reaches Nop but not Finish)
        assert!(!result.paths.is_empty());
        assert!(result
            .paths
            .iter()
            .all(|s| s.status == TaintPathStatus::Warning));
    }

    #[test]
    fn test_source_reaches_sink_dangerous() {
        // WaitEvent at 0 -> Finish at 1
        let parts = make_parts_with_next(vec![
            (
                CompiledNodeKind::WaitEvent {
                    event: SlotIdx::new(0),
                    timeout_slot: None,
                },
                Some(StepIdx::new(1)),
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts);
        assert_eq!(result.sources.len(), 1);
        assert_eq!(result.sinks.len(), 1);
        assert!(!result.finish_safe);
        assert!(result
            .paths
            .iter()
            .all(|s| s.status == TaintPathStatus::Dangerous));
    }

    #[test]
    fn test_ask_source_reaches_sink() {
        // Ask at 0 -> Finish at 1
        let parts = make_parts_with_next(vec![
            (
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(1),
                    timeout_slot: Some(SlotIdx::new(2)),
                },
                Some(StepIdx::new(1)),
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts);
        assert_eq!(result.sources.len(), 1);
        assert!(!result.finish_safe);
    }

    #[test]
    fn test_source_indirect_path_to_sink() {
        // WaitEvent at 0 -> Nop at 1 -> Finish at 2
        let parts = make_parts_with_next(vec![
            (
                CompiledNodeKind::WaitEvent {
                    event: SlotIdx::new(0),
                    timeout_slot: None,
                },
                Some(StepIdx::new(1)),
            ),
            (CompiledNodeKind::Nop, Some(StepIdx::new(2))),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts);
        assert_eq!(result.sources.len(), 1);
        assert!(!result.finish_safe);
        // Should reach both Nop and Finish
        let reachable_steps: Vec<StepIdx> = result.paths.iter().map(|s| s.to).collect();
        assert!(reachable_steps.iter().any(|s| *s == StepIdx::new(1)));
        assert!(reachable_steps.iter().any(|s| *s == StepIdx::new(2)));
    }

    #[test]
    fn test_multiple_sources() {
        // WaitEvent at 0, Ask at 1, both -> Finish at 2
        let parts = make_parts_with_next(vec![
            (
                CompiledNodeKind::WaitEvent {
                    event: SlotIdx::new(0),
                    timeout_slot: None,
                },
                Some(StepIdx::new(2)),
            ),
            (
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(1),
                    timeout_slot: None,
                },
                Some(StepIdx::new(2)),
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts);
        assert_eq!(result.sources.len(), 2);
        assert_eq!(result.sinks.len(), 1);
        assert!(!result.finish_safe);
    }

    #[test]
    fn test_no_next_from_source_still_listed_as_source() {
        // WaitEvent at 0 with no next, Finish at 1 (unreachable)
        let parts = make_parts_with_next(vec![
            (
                CompiledNodeKind::WaitEvent {
                    event: SlotIdx::new(0),
                    timeout_slot: None,
                },
                None,
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts);
        assert_eq!(result.sources.len(), 1);
        assert_eq!(result.sinks.len(), 1);
        // Source cannot reach sink because no next edge
        assert!(result.finish_safe);
        assert!(result.paths.is_empty());
    }

    #[test]
    fn test_walk_forward_respects_bounds() {
        // Chain: 0 -> 1 -> 2 (Finish)
        // WaitEvent at 0
        let parts = make_parts_with_next(vec![
            (
                CompiledNodeKind::WaitEvent {
                    event: SlotIdx::new(0),
                    timeout_slot: None,
                },
                Some(StepIdx::new(1)),
            ),
            (CompiledNodeKind::Nop, Some(StepIdx::new(2))),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
            ),
        ]);
        let reachable = walk_forward(&parts, StepIdx::new(0));
        assert_eq!(reachable.len(), 2);
        assert!(reachable.contains(&StepIdx::new(1)));
        assert!(reachable.contains(&StepIdx::new(2)));
    }

    #[test]
    fn test_walk_forward_no_cycles() {
        // Two nodes pointing at each other would be a cycle, but walk_forward
        // uses a visited set to avoid infinite loops.
        // 0 (Nop) -> 1 (Nop) -> 0 -- cycle
        // We test from an external source pointing at this.
        let parts = make_parts_with_next(vec![
            (
                CompiledNodeKind::WaitEvent {
                    event: SlotIdx::new(0),
                    timeout_slot: None,
                },
                Some(StepIdx::new(1)),
            ),
            (CompiledNodeKind::Nop, Some(StepIdx::new(0))),
        ]);
        let reachable = walk_forward(&parts, StepIdx::new(0));
        // Should reach node 1 but not re-visit node 0
        assert!(reachable.contains(&StepIdx::new(1)));
        // Should not loop -- visited set prevents revisiting start
        assert!(!reachable.contains(&StepIdx::new(0)));
    }

    #[test]
    fn test_empty_workflow() {
        let parts = WorkflowParts {
            name: String::from("empty").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Vec::new().into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = compute_taint_overlay(&parts);
        assert!(result.sources.is_empty());
        assert!(result.sinks.is_empty());
        assert!(result.paths.is_empty());
        assert!(result.finish_safe);
    }

    #[test]
    fn test_finish_only_no_sources() {
        let parts = make_parts_with_next(vec![(
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
        )]);
        let result = compute_taint_overlay(&parts);
        assert!(result.sources.is_empty());
        assert_eq!(result.sinks.len(), 1);
        assert!(result.finish_safe);
        assert!(result.paths.is_empty());
    }
}
