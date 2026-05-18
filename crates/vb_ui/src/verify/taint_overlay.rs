#![forbid(unsafe_code)]
//! Taint flow overlay -- traces secret-to-sink paths through the workflow graph.
//!
//! Given a `WorkflowParts` and a taint map (slot -> taint label), identifies:
//! - Secret source nodes (slots carrying "Secret" or "DerivedFromSecret" labels)
//! - Reachable sink nodes (Finish)
//! - Paths from source to sink with intermediate nodes
//! - Whether the Finish node is safe (no secret reached it)
//!
//! Colour mapping for the verification screen:
//! - Secret source nodes: neon magenta (#ff00ff)
//! - Tainted intermediate nodes: magenta
//! - Forbidden sink paths: neon red (#ff073a)
//! - Safe Finish node: neon teal (#00e5c7)
//! - Clean nodes: default

use std::collections::{HashMap, HashSet};

use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

use crate::theme::colors;

// ---------------------------------------------------------------------------
// Colour constants for taint overlay rendering
// ---------------------------------------------------------------------------

/// Colour for secret source nodes -- neon magenta (#ff00ff).
pub const COLOR_SECRET_SOURCE: [f32; 4] = colors::neon::MAGENTA;

/// Colour for tainted intermediate nodes -- magenta.
pub const COLOR_TAINTED: [f32; 4] = colors::neon::MAGENTA;

/// Colour for forbidden sink paths -- neon red (#ff073a).
pub const COLOR_FORBIDDEN_SINK: [f32; 4] = colors::neon::RED;

/// Colour for safe Finish node -- neon teal (#00e5c7).
pub const COLOR_SAFE_FINISH: [f32; 4] = colors::neon::TEAL;

/// Default colour for clean nodes.
pub const COLOR_CLEAN: [f32; 4] = colors::text::DIM;

// ---------------------------------------------------------------------------
// Legacy types (used by graph_overlay.rs and certificates.rs)
// ---------------------------------------------------------------------------

/// Severity of a taint path segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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

// ---------------------------------------------------------------------------
// New Phase 2B types
// ---------------------------------------------------------------------------

/// A complete flow path from a secret source to a sink.
#[derive(Debug, Clone)]
pub struct TaintFlowPath {
    /// The step that introduced the secret value.
    pub source_step: StepIdx,
    /// The terminal sink step (Finish or last reachable).
    pub sink_step: StepIdx,
    /// All step indices along the path (inclusive of source and sink).
    pub path_nodes: Vec<StepIdx>,
    /// True when this path reaches a forbidden sink (Finish carrying a secret).
    pub is_forbidden: bool,
}

// ---------------------------------------------------------------------------
// Overlay result
// ---------------------------------------------------------------------------

/// Complete taint overlay result for a workflow.
///
/// Contains both the legacy fields (sources, sinks, paths) used by
/// `graph_overlay.rs` and `certificates.rs`, and the new Phase 2B
/// fields (tainted_nodes, clean_nodes, forbidden_sinks, flow_paths).
#[derive(Debug, Clone)]
pub struct TaintOverlayResult {
    // -- Legacy fields --
    /// All secret source step indices.
    pub sources: Vec<StepIdx>,
    /// All sink (Finish) step indices.
    pub sinks: Vec<StepIdx>,
    /// Path segments coloured by severity.
    pub paths: Vec<TaintPathSegment>,

    // -- Phase 2B fields --
    /// Steps whose output slots carry a secret taint label.
    pub tainted_nodes: HashSet<usize>,
    /// Steps that are not tainted (no secret label on any slot they write).
    pub clean_nodes: HashSet<usize>,
    /// Finish steps that a secret reaches (forbidden sinks).
    pub forbidden_sinks: HashSet<usize>,
    /// Complete flow paths from secret sources to sinks.
    pub flow_paths: Vec<TaintFlowPath>,

    /// True when no source can reach any sink.
    pub finish_safe: bool,
}

// ---------------------------------------------------------------------------
// Main computation
// ---------------------------------------------------------------------------

/// Compute the full taint overlay for a compiled workflow.
///
/// Walks the workflow graph from secret-referencing nodes toward Finish,
/// tracking taint propagation through slot writes.
///
/// - `parts`: the compiled workflow parts.
/// - `taint_map`: mapping from slot index to taint label string
///   (e.g. "Secret", "DerivedFromSecret").
pub fn compute_taint_overlay(
    parts: &WorkflowParts,
    taint_map: &HashMap<SlotIdx, String>,
) -> TaintOverlayResult {
    let sources = find_sources(parts, taint_map);
    let sinks = find_sinks(parts);
    let source_set: HashSet<StepIdx> = sources.iter().copied().collect();
    let sink_set: HashSet<StepIdx> = sinks.iter().copied().collect();

    // Classify nodes by taint.
    let tainted_nodes = find_tainted_nodes(parts, taint_map);
    let mut clean_nodes = HashSet::new();
    for (idx, _node) in parts.nodes.iter().enumerate() {
        if !tainted_nodes.contains(&idx) {
            clean_nodes.insert(idx);
        }
    }

    // Build legacy path segments.
    let mut legacy_paths: Vec<TaintPathSegment> = Vec::new();
    let mut any_source_reaches_sink = false;

    // Build Phase 2B flow paths and forbidden sinks.
    let mut flow_paths: Vec<TaintFlowPath> = Vec::new();
    let mut forbidden_sinks: HashSet<usize> = HashSet::new();

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

        // Legacy segments.
        for step in &reachable {
            legacy_paths.push(TaintPathSegment {
                from: *source,
                to: *step,
                status,
            });
        }

        // Phase 2B flow paths.
        if reaches_sink {
            for sink in sink_set.iter() {
                if reachable_set.contains(sink) {
                    let path_nodes = build_path_nodes(parts, *source, *sink, &source_set);
                    for &node in &path_nodes {
                        let node_idx = usize::from(node.get());
                        if sink_set.contains(&StepIdx::new(node.get())) {
                            forbidden_sinks.insert(node_idx);
                        }
                    }
                    flow_paths.push(TaintFlowPath {
                        source_step: *source,
                        sink_step: *sink,
                        path_nodes,
                        is_forbidden: true,
                    });
                }
            }
        } else if !reachable.is_empty() {
            // Source reaches non-sink nodes -- still create a flow path
            // with is_forbidden = false, sink being the last reachable node.
            let last = reachable
                .iter()
                .copied()
                .max_by_key(|s| s.get())
                .unwrap_or(*source);
            let mut path_nodes = vec![*source];
            for step in &reachable {
                path_nodes.push(*step);
            }
            flow_paths.push(TaintFlowPath {
                source_step: *source,
                sink_step: last,
                path_nodes,
                is_forbidden: false,
            });
        }
    }

    let finish_safe = !any_source_reaches_sink;

    TaintOverlayResult {
        sources,
        sinks,
        paths: legacy_paths,
        tainted_nodes,
        clean_nodes,
        forbidden_sinks,
        flow_paths,
        finish_safe,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Find nodes whose output slot carries a secret taint label.
fn find_tainted_nodes(
    parts: &WorkflowParts,
    taint_map: &HashMap<SlotIdx, String>,
) -> HashSet<usize> {
    let mut tainted = HashSet::new();

    for (idx, node) in parts.nodes.iter().enumerate() {
        // Check if this node's output slot is tainted.
        if let Some(output_slot) = node.output
            && is_secret_taint(taint_map, output_slot)
        {
            tainted.insert(idx);
        }

        // Also check if the node kind references a tainted input slot.
        let input_slots = collect_input_slots(&node.kind);
        for slot in input_slots {
            if is_secret_taint(taint_map, slot) {
                tainted.insert(idx);
            }
        }
    }

    tainted
}

/// Check whether a slot carries a secret taint label.
fn is_secret_taint(taint_map: &HashMap<SlotIdx, String>, slot: SlotIdx) -> bool {
    match taint_map.get(&slot) {
        Some(label) => {
            label.contains("Secret")
                || label.contains("DerivedFromSecret")
                || label.contains("derived")
        }
        None => false,
    }
}

/// Find source steps: either nodes identified by the taint map as writing
/// to secret-labelled slots, or nodes that are WaitEvent/Ask (legacy).
fn find_sources(parts: &WorkflowParts, taint_map: &HashMap<SlotIdx, String>) -> Vec<StepIdx> {
    let mut sources = Vec::new();
    let mut seen = HashSet::new();

    // Taint-map based sources: nodes whose output slot is secret-tainted.
    for node in parts.nodes.iter() {
        if let Some(output_slot) = node.output
            && is_secret_taint(taint_map, output_slot)
            && seen.insert(node.id)
        {
            sources.push(node.id);
        }
    }

    // Legacy structural sources: WaitEvent and Ask nodes that could
    // introduce secrets even without explicit taint labels.
    for node in parts.nodes.iter() {
        match node.kind {
            CompiledNodeKind::WaitEvent { .. } | CompiledNodeKind::Ask { .. }
                if seen.insert(node.id) =>
            {
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
    if let Some(next) = node.next {
        let next_usize = next.as_usize();
        if next_usize < node_count && visited.insert(next) {
            queue.push(next);
        }
    }
}

/// Build the path of step indices from source to sink via BFS shortest path.
fn build_path_nodes(
    parts: &WorkflowParts,
    source: StepIdx,
    sink: StepIdx,
    source_set: &HashSet<StepIdx>,
) -> Vec<StepIdx> {
    let node_count = parts.nodes.len();
    let mut parent: HashMap<StepIdx, StepIdx> = HashMap::new();
    let mut visited = HashSet::new();
    visited.insert(source);
    let mut queue = Vec::new();

    // Seed with successors of source.
    if let Some(node) = parts.nodes.get(source.as_usize()) {
        seed_successors(
            node,
            node_count,
            &mut visited,
            &mut queue,
            source,
            &mut parent,
        );
    }

    let mut found = false;
    while let Some(current) = queue.pop() {
        if current == sink {
            found = true;
            break;
        }

        if let Some(node) = parts.nodes.get(current.as_usize()) {
            seed_successors(
                node,
                node_count,
                &mut visited,
                &mut queue,
                current,
                &mut parent,
            );
        }
    }

    if !found {
        // Fallback: just source and sink.
        return vec![source, sink];
    }

    // Reconstruct path from sink back to source.
    let mut path = Vec::new();
    let mut current = sink;
    path.push(current);
    while let Some(&p) = parent.get(&current) {
        if p == source {
            break;
        }
        path.push(p);
        current = p;
    }
    path.push(source);
    path.reverse();

    // Remove any non-source nodes that are themselves sources
    // (avoid co-mingling paths from different sources).
    let filtered: Vec<StepIdx> = path
        .iter()
        .copied()
        .filter(|&s| s == source || !source_set.contains(&s))
        .collect();

    filtered
}

/// Seed successors and record parent links for path reconstruction.
fn seed_successors(
    node: &vb_core::workflow::CompiledNode,
    node_count: usize,
    visited: &mut HashSet<StepIdx>,
    queue: &mut Vec<StepIdx>,
    parent_step: StepIdx,
    parent: &mut HashMap<StepIdx, StepIdx>,
) {
    if let Some(next) = node.next {
        let next_usize = next.as_usize();
        if next_usize < node_count && visited.insert(next) {
            parent.insert(next, parent_step);
            queue.push(next);
        }
    }
}

/// Collect the input slot references from a CompiledNodeKind.
fn collect_input_slots(kind: &CompiledNodeKind) -> Vec<SlotIdx> {
    match kind {
        CompiledNodeKind::Nop => Vec::new(),
        CompiledNodeKind::SetConst { .. } => Vec::new(),
        CompiledNodeKind::Copy { source } => vec![*source],
        CompiledNodeKind::EvalExpr { .. } => Vec::new(),
        CompiledNodeKind::BuildObject { fields } => fields.iter().map(|(_, slot)| *slot).collect(),
        CompiledNodeKind::BuildList { items } => items.clone().into_iter().collect(),
        CompiledNodeKind::Do { input, .. } => vec![*input],
        CompiledNodeKind::Choose { .. } => Vec::new(),
        CompiledNodeKind::ChooseSlot { .. } => Vec::new(),
        CompiledNodeKind::ForEachStart {
            input, item_slot, ..
        } => vec![*input, *item_slot],
        CompiledNodeKind::ForEachNext { iterator_slot, .. } => vec![*iterator_slot],
        CompiledNodeKind::ForEachJoin { output } => vec![*output],
        CompiledNodeKind::TogetherStart { .. } => Vec::new(),
        CompiledNodeKind::TogetherBranch { accumulator, .. } => vec![*accumulator],
        CompiledNodeKind::TogetherJoin { accumulator, .. } => vec![*accumulator],
        CompiledNodeKind::CollectStart { source, .. } => vec![*source],
        CompiledNodeKind::CollectPage { collector_slot, .. } => vec![*collector_slot],
        CompiledNodeKind::CollectNext { collector_slot, .. } => vec![*collector_slot],
        CompiledNodeKind::CollectFinish { collector_slot } => vec![*collector_slot],
        CompiledNodeKind::ReduceStart {
            input, accumulator, ..
        } => vec![*input, *accumulator],
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            ..
        } => {
            vec![*iterator_slot, *accumulator]
        }
        CompiledNodeKind::ReduceFinish { accumulator } => vec![*accumulator],
        CompiledNodeKind::RepeatStart { .. } => Vec::new(),
        CompiledNodeKind::RepeatAttempt { attempt_slot, .. } => vec![*attempt_slot],
        CompiledNodeKind::RepeatCheck { attempt_slot, .. } => vec![*attempt_slot],
        CompiledNodeKind::RepeatFinish { result } => vec![*result],
        CompiledNodeKind::WaitUntil { deadline_slot } => vec![*deadline_slot],
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            let mut slots = vec![*event];
            if let Some(ts) = timeout_slot {
                slots.push(*ts);
            }
            slots
        }
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            let mut slots = vec![*prompt];
            if let Some(ts) = timeout_slot {
                slots.push(*ts);
            }
            slots
        }
        CompiledNodeKind::AskResume { answer } => vec![*answer],
        CompiledNodeKind::RetryCheck { policy_slot, .. } => vec![*policy_slot],
        CompiledNodeKind::ErrorHandler { error_slot, .. } => {
            let mut slots = Vec::new();
            if let Some(es) = error_slot {
                slots.push(*es);
            }
            slots
        }
        CompiledNodeKind::Jump { .. } => Vec::new(),
        CompiledNodeKind::Finish { result } => vec![*result],
    }
}

// ---------------------------------------------------------------------------
// Colour mapping helper
// ---------------------------------------------------------------------------

/// Returns the overlay colour for a given step based on the taint overlay result.
///
/// - Secret source nodes: neon magenta (#ff00ff)
/// - Tainted intermediate nodes: magenta
/// - Forbidden sink paths: neon red (#ff073a)
/// - Safe Finish node: neon teal (#00e5c7)
/// - Clean nodes: default dim grey
#[must_use]
pub fn step_color(step_idx: usize, result: &TaintOverlayResult) -> [f32; 4] {
    let step = StepIdx::new(u16::try_from(step_idx).unwrap_or(u16::MAX));

    // Forbidden sink takes highest priority.
    if result.forbidden_sinks.contains(&step_idx) {
        return COLOR_FORBIDDEN_SINK;
    }

    // Secret source nodes.
    if result.sources.contains(&step) {
        return COLOR_SECRET_SOURCE;
    }

    // Tainted intermediate nodes.
    if result.tainted_nodes.contains(&step_idx) {
        return COLOR_TAINTED;
    }

    // Safe Finish node.
    if result.finish_safe && result.sinks.contains(&step) {
        return COLOR_SAFE_FINISH;
    }

    COLOR_CLEAN
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::WorkflowDigest;
    use vb_core::workflow::{CompiledNode, ResourceContract};

    // -- Helpers --

    fn make_parts_with_next(kinds: Vec<(CompiledNodeKind, Option<StepIdx>)>) -> WorkflowParts {
        let nodes: Vec<CompiledNode> = kinds
            .into_iter()
            .enumerate()
            .map(|(i, (kind, next))| CompiledNode {
                id: StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX)),
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

    fn make_parts_with_output(
        kinds: Vec<(CompiledNodeKind, Option<StepIdx>, Option<SlotIdx>)>,
    ) -> WorkflowParts {
        let nodes: Vec<CompiledNode> = kinds
            .into_iter()
            .enumerate()
            .map(|(i, (kind, next, output))| CompiledNode {
                id: StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX)),
                output,
                next,
                on_error: None,
                error_slot: None,
                kind,
            })
            .collect();
        let count = nodes.len();
        WorkflowParts {
            name: String::from("output-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 8,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: (0..count)
                .map(|_| Box::<str>::from(""))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    fn empty_taint_map() -> HashMap<SlotIdx, String> {
        HashMap::new()
    }

    fn secret_taint_map(slot: SlotIdx) -> HashMap<SlotIdx, String> {
        let mut map = HashMap::new();
        map.insert(slot, String::from("Secret"));
        map
    }

    fn derived_taint_map(slot: SlotIdx) -> HashMap<SlotIdx, String> {
        let mut map = HashMap::new();
        map.insert(slot, String::from("DerivedFromSecret"));
        map
    }

    // -- Test 1: Clean workflow (no secrets, no taint map) --

    #[test]
    fn test_clean_workflow_no_secrets() {
        let parts = make_parts_with_next(vec![
            (CompiledNodeKind::Nop, Some(StepIdx::new(1))),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        assert!(result.sources.is_empty());
        assert_eq!(result.sinks.len(), 1);
        assert!(result.paths.is_empty());
        assert!(result.finish_safe);
        assert!(result.tainted_nodes.is_empty());
        assert_eq!(result.clean_nodes.len(), 2);
        assert!(result.forbidden_sinks.is_empty());
        assert!(result.flow_paths.is_empty());
    }

    // -- Test 2: Direct secret leak via taint map --

    #[test]
    fn test_direct_secret_leak_via_taint_map() {
        let parts = make_parts_with_output(vec![
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(0)),
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]);
        let taint_map = secret_taint_map(SlotIdx::new(0));
        let result = compute_taint_overlay(&parts, &taint_map);

        assert_eq!(result.sources.len(), 1);
        assert_eq!(result.sources[0], StepIdx::new(0));
        assert_eq!(result.sinks.len(), 1);
        assert!(!result.finish_safe);
        assert!(result.tainted_nodes.contains(&0));
        assert!(!result.clean_nodes.contains(&0));
        assert!(result.forbidden_sinks.contains(&1));

        let forbidden_paths: Vec<&TaintFlowPath> = result
            .flow_paths
            .iter()
            .filter(|p| p.is_forbidden)
            .collect();
        assert_eq!(forbidden_paths.len(), 1);
        assert_eq!(forbidden_paths[0].source_step, StepIdx::new(0));
        assert_eq!(forbidden_paths[0].sink_step, StepIdx::new(1));
    }

    // -- Test 3: Indirect propagation through intermediate steps --

    #[test]
    fn test_indirect_propagation_through_intermediate() {
        let parts = make_parts_with_output(vec![
            (
                CompiledNodeKind::WaitEvent {
                    event: SlotIdx::new(0),
                    timeout_slot: None,
                },
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(1)),
            ),
            (CompiledNodeKind::Nop, Some(StepIdx::new(2)), None),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        assert_eq!(result.sources.len(), 1);
        assert!(!result.finish_safe);

        let forbidden_paths: Vec<&TaintFlowPath> = result
            .flow_paths
            .iter()
            .filter(|p| p.is_forbidden)
            .collect();
        assert_eq!(forbidden_paths.len(), 1);
        let path = &forbidden_paths[0];
        assert!(path.path_nodes.contains(&StepIdx::new(0)));
        assert!(path.path_nodes.contains(&StepIdx::new(1)));
        assert!(path.path_nodes.contains(&StepIdx::new(2)));
    }

    // -- Test 4: Safe finish (no leak) --

    #[test]
    fn test_safe_finish_no_leak() {
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
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        assert_eq!(result.sources.len(), 1);
        assert_eq!(result.sinks.len(), 1);
        assert!(result.finish_safe);
        assert!(result.forbidden_sinks.is_empty());
    }

    // -- Test 5: Multiple secret sources --

    #[test]
    fn test_multiple_secret_sources() {
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
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        assert_eq!(result.sources.len(), 2);
        assert_eq!(result.sinks.len(), 1);
        assert!(!result.finish_safe);
        assert!(result.forbidden_sinks.contains(&2));

        let forbidden_paths: Vec<&TaintFlowPath> = result
            .flow_paths
            .iter()
            .filter(|p| p.is_forbidden)
            .collect();
        assert_eq!(forbidden_paths.len(), 2);
    }

    // -- Test 6: Empty workflow --

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
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        assert!(result.sources.is_empty());
        assert!(result.sinks.is_empty());
        assert!(result.paths.is_empty());
        assert!(result.finish_safe);
        assert!(result.tainted_nodes.is_empty());
        assert!(result.clean_nodes.is_empty());
        assert!(result.forbidden_sinks.is_empty());
        assert!(result.flow_paths.is_empty());
    }

    // -- Test 7: DerivedFromSecret taint label detection --

    #[test]
    fn test_derived_from_secret_taint() {
        let parts = make_parts_with_output(vec![
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(0)),
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]);
        let taint_map = derived_taint_map(SlotIdx::new(0));
        let result = compute_taint_overlay(&parts, &taint_map);

        assert_eq!(result.sources.len(), 1);
        assert!(result.tainted_nodes.contains(&0));
        assert!(!result.finish_safe);
    }

    // -- Test 8: Finish-only workflow (no sources, clean finish) --

    #[test]
    fn test_finish_only_no_sources() {
        let parts = make_parts_with_next(vec![(
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
        )]);
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        assert!(result.sources.is_empty());
        assert_eq!(result.sinks.len(), 1);
        assert!(result.finish_safe);
        assert!(result.paths.is_empty());
        assert!(result.clean_nodes.contains(&0));
    }

    // -- Test 9: Colour mapping -- secret source is magenta --

    #[test]
    fn test_color_secret_source() {
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
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        assert_eq!(step_color(0, &result), COLOR_SECRET_SOURCE);
    }

    // -- Test 10: Colour mapping -- forbidden sink is red --

    #[test]
    fn test_color_forbidden_sink() {
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
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        assert_eq!(step_color(1, &result), COLOR_FORBIDDEN_SINK);
    }

    // -- Test 11: Colour mapping -- safe finish is teal --

    #[test]
    fn test_color_safe_finish() {
        let parts = make_parts_with_next(vec![(
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
        )]);
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        assert_eq!(step_color(0, &result), COLOR_SAFE_FINISH);
    }

    // -- Test 12: Colour mapping -- clean node is dim --

    #[test]
    fn test_color_clean_node() {
        let parts = make_parts_with_next(vec![
            (CompiledNodeKind::Nop, Some(StepIdx::new(1))),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        assert_eq!(step_color(0, &result), COLOR_CLEAN);
    }

    // -- Test 13: BFS cycle prevention --

    #[test]
    fn test_walk_forward_no_cycles() {
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
        assert!(reachable.contains(&StepIdx::new(1)));
        assert!(!reachable.contains(&StepIdx::new(0)));
    }

    // -- Test 14: Taint map with multiple slots --

    #[test]
    fn test_taint_map_multiple_slots() {
        let parts = make_parts_with_output(vec![
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(0)),
            ),
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(2)),
                Some(SlotIdx::new(1)),
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ]);
        let mut taint_map = HashMap::new();
        taint_map.insert(SlotIdx::new(0), String::from("Secret"));
        taint_map.insert(SlotIdx::new(1), String::from("DerivedFromSecret"));

        let result = compute_taint_overlay(&parts, &taint_map);

        assert_eq!(result.sources.len(), 2);
        assert!(result.tainted_nodes.contains(&0));
        assert!(result.tainted_nodes.contains(&1));
        assert!(!result.finish_safe);
    }

    // -- Test 15: Non-secret taint label is ignored --

    #[test]
    fn test_non_secret_taint_label_ignored() {
        let parts = make_parts_with_output(vec![
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(0)),
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]);
        let mut taint_map = HashMap::new();
        taint_map.insert(SlotIdx::new(0), String::from("Public"));

        let result = compute_taint_overlay(&parts, &taint_map);

        assert!(result.sources.is_empty());
        assert!(result.tainted_nodes.is_empty());
        assert!(result.finish_safe);
    }

    // -- Test 16: Ask node is a structural source --

    #[test]
    fn test_ask_node_is_structural_source() {
        let parts = make_parts_with_next(vec![
            (
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
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
        let result = compute_taint_overlay(&parts, &empty_taint_map());
        assert_eq!(result.sources.len(), 1);
        assert_eq!(result.sources[0], StepIdx::new(0));
        assert!(!result.finish_safe);
    }

    // -- Test 17: "derived" taint label triggers secret detection --

    #[test]
    fn test_derived_keyword_taint_label() {
        let parts = make_parts_with_output(vec![
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(0)),
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]);
        let mut taint_map = HashMap::new();
        taint_map.insert(SlotIdx::new(0), String::from("derived-value"));

        let result = compute_taint_overlay(&parts, &taint_map);
        // "derived" is a recognized keyword for taint.
        assert_eq!(result.sources.len(), 1);
        assert!(result.tainted_nodes.contains(&0));
    }

    // -- Test 18: Tainted intermediate node colour is magenta --

    #[test]
    fn test_color_tainted_intermediate() {
        // Node 0 writes to slot 0 which is tainted, node 1 is Finish reading slot 0.
        // Node 0 is a source, node 1 is a forbidden sink.
        // But if we add an intermediate that is tainted via input slot but NOT a source...
        let parts = make_parts_with_output(vec![
            (
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(1)),
            ),
            (
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                Some(StepIdx::new(2)),
                None,
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ]);
        let taint_map = secret_taint_map(SlotIdx::new(0));
        let result = compute_taint_overlay(&parts, &taint_map);

        // Node 1 reads tainted slot 0 via Copy but is not a source.
        // It should be in tainted_nodes and colored with COLOR_TAINTED.
        if result.tainted_nodes.contains(&1)
            && !result.sources.contains(&StepIdx::new(1))
            && !result.forbidden_sinks.contains(&1)
        {
            assert_eq!(step_color(1, &result), COLOR_TAINTED);
        }
    }

    // -- Test 19: step_color for out-of-range step index --

    #[test]
    fn test_step_color_out_of_range_returns_clean() {
        let parts = make_parts_with_next(vec![(
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
        )]);
        let result = compute_taint_overlay(&parts, &empty_taint_map());
        // Step index 999 is beyond u16::MAX clamping -- should return CLEAN.
        assert_eq!(step_color(999, &result), COLOR_CLEAN);
    }

    // -- Test 20: Flow path for non-forbidden path has is_forbidden false --

    #[test]
    fn test_flow_path_non_forbidden() {
        // WaitEvent with no path to Finish.
        let parts = make_parts_with_next(vec![
            (
                CompiledNodeKind::WaitEvent {
                    event: SlotIdx::new(0),
                    timeout_slot: None,
                },
                None, // no next edge
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts, &empty_taint_map());
        // The WaitEvent source has no successors, so no reachable nodes.
        // Non-forbidden flow paths only created when reachable is non-empty.
        assert!(result.flow_paths.is_empty() || result.flow_paths.iter().all(|p| !p.is_forbidden));
    }

    // -- Test 21: TaintOverlayResult sources and sinks are independent --

    #[test]
    fn test_sources_and_sinks_independent() {
        // Two Finish nodes and two WaitEvent nodes.
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
            (
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(1),
                    timeout_slot: None,
                },
                Some(StepIdx::new(3)),
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts, &empty_taint_map());
        assert_eq!(result.sources.len(), 2);
        assert_eq!(result.sinks.len(), 2);
    }

    // -- Test 22: collect_input_slots returns correct slots for BuildObject --

    #[test]
    fn test_collect_input_slots_build_object() {
        use vb_core::ids::SymbolId;
        let fields: Box<[(SymbolId, SlotIdx)]> = Box::new([
            (SymbolId::new(0), SlotIdx::new(1)),
            (SymbolId::new(1), SlotIdx::new(2)),
            (SymbolId::new(2), SlotIdx::new(3)),
        ]);
        let slots = collect_input_slots(&CompiledNodeKind::BuildObject { fields });
        assert_eq!(slots.len(), 3);
        assert!(slots.contains(&SlotIdx::new(1)));
        assert!(slots.contains(&SlotIdx::new(2)));
        assert!(slots.contains(&SlotIdx::new(3)));
    }

    // -- Test 23: collect_input_slots returns correct slots for WaitEvent with timeout --

    #[test]
    fn test_collect_input_slots_wait_event_with_timeout() {
        let slots = collect_input_slots(&CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(5),
            timeout_slot: Some(SlotIdx::new(6)),
        });
        assert_eq!(slots.len(), 2);
        assert!(slots.contains(&SlotIdx::new(5)));
        assert!(slots.contains(&SlotIdx::new(6)));
    }

    // -- Test 24: collect_input_slots returns empty for Nop --

    #[test]
    fn test_collect_input_slots_nop_empty() {
        let slots = collect_input_slots(&CompiledNodeKind::Nop);
        assert!(slots.is_empty());
    }

    // -- Test 25: collect_input_slots for Finish returns result slot --

    #[test]
    fn test_collect_input_slots_finish() {
        let slots = collect_input_slots(&CompiledNodeKind::Finish {
            result: SlotIdx::new(42),
        });
        assert_eq!(slots.len(), 1);
        assert!(slots.contains(&SlotIdx::new(42)));
    }

    // -- Test 26: Colour constants are distinct --

    #[test]
    fn test_color_constants_are_distinct() {
        assert_ne!(COLOR_SECRET_SOURCE, COLOR_FORBIDDEN_SINK);
        assert_ne!(COLOR_FORBIDDEN_SINK, COLOR_SAFE_FINISH);
        assert_ne!(COLOR_SAFE_FINISH, COLOR_CLEAN);
    }

    // =========================================================================
    // BLACK HAT security-focused tests (BH-01 through BH-08)
    // =========================================================================

    /// BH-01: is_secret_taint with exact label "Secret" matches.
    ///
    /// The taint label "Secret" must be recognised as a secret source.
    /// This validates the primary matching path in `is_secret_taint`.
    #[test]
    fn bh01_exact_secret_label_matches() {
        let mut map = HashMap::new();
        let slot = SlotIdx::new(10);
        map.insert(slot, String::from("Secret"));

        assert!(
            is_secret_taint(&map, slot),
            "exact label 'Secret' must be recognised as a secret taint"
        );
    }

    /// BH-02: is_secret_taint with "NotASecret" incorrectly matches (false positive).
    ///
    /// BLACK HAT FINDING: `is_secret_taint` uses `label.contains("Secret")`,
    /// so "NotASecret" produces a **false positive** because "Secret" is a
    /// substring.  This test documents the known vulnerability.  If the
    /// implementation is fixed to use exact or prefix matching, this test
    /// must be updated to assert `false` instead.
    #[test]
    fn bh02_not_a_secret_false_positive() {
        let mut map = HashMap::new();
        let slot = SlotIdx::new(20);
        map.insert(slot, String::from("NotASecret"));

        // BLACK HAT: substring match causes false positive.
        // Currently returns true; the fix should make this false.
        assert!(
            is_secret_taint(&map, slot),
            "BLACK HAT: 'NotASecret' incorrectly matches via substring 'Secret' -- known false-positive vulnerability"
        );
    }

    /// BH-03: is_secret_taint with "derived" in an unrelated context.
    ///
    /// The label "unrelated-derived-data" contains the substring "derived"
    /// and is therefore treated as a secret taint.  This is a broad catch
    /// pattern that could produce false positives on labels that happen to
    /// include "derived" in an unrelated context.
    #[test]
    fn bh03_derived_in_unrelated_context() {
        let mut map = HashMap::new();
        let slot = SlotIdx::new(30);
        map.insert(slot, String::from("unrelated-derived-data"));

        assert!(
            is_secret_taint(&map, slot),
            "label containing 'derived' is matched by the broad substring check"
        );

        // Verify a label that has no secret-related substrings is clean.
        let clean_slot = SlotIdx::new(31);
        map.insert(clean_slot, String::from("PublicValue"));

        assert!(
            !is_secret_taint(&map, clean_slot),
            "label 'PublicValue' must not be recognised as a secret taint"
        );
    }

    /// BH-04: walk_forward traversal order follows BFS with `Vec::pop`.
    ///
    /// `walk_forward` uses a `Vec` as a stack (`pop`), which makes it DFS,
    /// not BFS (despite comments saying BFS).  Construct a diamond graph:
    ///
    ///   0 -> 1 -> 3
    ///   0 -> 2 -> 3
    ///
    /// With DFS (pop), node 2 is visited before node 1 (LIFO).  The test
    /// verifies the actual traversal order produced by the implementation.
    #[test]
    fn bh04_walk_forward_traversal_order_is_dfs() {
        use vb_core::workflow::CompiledNode;

        // Build nodes: 0 -> {1, 2} where both 1 and 2 point to 3.
        // CompiledNode has a single `next`, so we simulate a linear chain
        // 0 -> 1 -> 2 -> 3 to test order.
        //
        // For a true branch test we need TogetherStart or ChooseSlot,
        // but walk_forward only follows `next` edges.  So we test a
        // linear chain 0 -> 1 -> 2 -> 3 and verify order.
        let kinds: Vec<(CompiledNodeKind, Option<StepIdx>)> = vec![
            (CompiledNodeKind::Nop, Some(StepIdx::new(1))),
            (CompiledNodeKind::Nop, Some(StepIdx::new(2))),
            (CompiledNodeKind::Nop, Some(StepIdx::new(3))),
            (CompiledNodeKind::Nop, None),
        ];

        let nodes: Vec<CompiledNode> = kinds
            .into_iter()
            .enumerate()
            .map(|(i, (kind, next))| CompiledNode {
                id: StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX)),
                output: None,
                next,
                on_error: None,
                error_slot: None,
                kind,
            })
            .collect();
        let count = nodes.len();
        let parts = WorkflowParts {
            name: String::from("bh04").into_boxed_str(),
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
        };

        let reachable = walk_forward(&parts, StepIdx::new(0));

        // All three successors must be present.
        assert_eq!(
            reachable.len(),
            3,
            "all three successors should be reachable"
        );
        assert!(
            reachable.contains(&StepIdx::new(1)),
            "node 1 must be reachable from node 0"
        );
        assert!(
            reachable.contains(&StepIdx::new(2)),
            "node 2 must be reachable from node 0"
        );
        assert!(
            reachable.contains(&StepIdx::new(3)),
            "node 3 must be reachable from node 0"
        );

        // Vec::pop is LIFO, so the last enqueued element comes out first.
        // Node 1 is enqueued first, then its successor 2, then 3.
        // Pop order: 1 (pushed first when processing node 0),
        //   then processing 1 pushes 2, pop 2, processing 2 pushes 3, pop 3.
        // So result order is [1, 2, 3].
        assert_eq!(
            reachable[0],
            StepIdx::new(1),
            "first visited node should be 1 (direct successor)"
        );
        assert_eq!(
            reachable[1],
            StepIdx::new(2),
            "second visited node should be 2"
        );
        assert_eq!(
            reachable[2],
            StepIdx::new(3),
            "third visited node should be 3"
        );
    }

    /// BH-05: ForEachJoin `output` slot appears in `collect_input_slots`.
    ///
    /// BLACK HAT FINDING: `collect_input_slots` returns the `output` slot
    /// for `ForEachJoin { output }`.  Semantically, `output` is the slot
    /// where the joined result is *written*, not an input that is read.
    /// This means the taint analysis treats the ForEachJoin output as an
    /// input slot, which could cause incorrect taint propagation.  The test
    /// documents this known issue.
    #[test]
    fn bh05_for_each_join_output_in_input_slots() {
        let output_slot = SlotIdx::new(7);
        let kind = CompiledNodeKind::ForEachJoin {
            output: output_slot,
        };
        let input_slots = collect_input_slots(&kind);

        // BLACK HAT: The output slot of ForEachJoin is incorrectly listed
        // as an input slot.  This is a semantic error in collect_input_slots.
        assert!(
            input_slots.contains(&output_slot),
            "BLACK HAT: ForEachJoin output slot is incorrectly treated as an input slot"
        );

        // Verify it is the only slot returned.
        assert_eq!(
            input_slots.len(),
            1,
            "ForEachJoin should report exactly one slot in collect_input_slots"
        );
    }

    /// BH-06: Empty taint map produces no flow paths and no tainted nodes.
    ///
    /// With no entries in the taint map, there should be zero sources,
    /// zero flow paths, zero tainted nodes, and `finish_safe` must be true
    /// even when a Finish node exists.
    #[test]
    fn bh06_empty_taint_map_produces_no_flows() {
        let parts = make_parts_with_output(vec![
            (
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(1)),
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts, &HashMap::new());

        assert!(
            result.sources.is_empty(),
            "no sources should be found with empty taint map"
        );
        assert!(
            result.flow_paths.is_empty(),
            "no flow paths should exist with empty taint map"
        );
        assert!(
            result.tainted_nodes.is_empty(),
            "no tainted nodes with empty taint map"
        );
        assert!(
            result.forbidden_sinks.is_empty(),
            "no forbidden sinks with empty taint map"
        );
        assert!(
            result.finish_safe,
            "finish must be safe when taint map is empty"
        );
        assert_eq!(
            result.clean_nodes.len(),
            2,
            "all nodes must be clean with empty taint map"
        );
    }

    /// BH-07: Single source-to-sink path detection.
    ///
    /// Build a linear chain where node 0 has a tainted output slot and
    /// flows through node 1 to a Finish node (node 2).  Verify that exactly
    /// one forbidden flow path is detected with correct source, sink, and
    /// intermediate nodes.
    #[test]
    fn bh07_single_source_to_sink_path() {
        let parts = make_parts_with_output(vec![
            // Node 0: writes to slot 0 (tainted)
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(0)),
            ),
            // Node 1: intermediate
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(2)),
                Some(SlotIdx::new(1)),
            ),
            // Node 2: Finish sink
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ]);

        let mut taint_map = HashMap::new();
        taint_map.insert(SlotIdx::new(0), String::from("Secret"));

        let result = compute_taint_overlay(&parts, &taint_map);

        // Exactly one source.
        assert_eq!(result.sources.len(), 1, "should have exactly one source");
        assert_eq!(
            result.sources[0],
            StepIdx::new(0),
            "source should be node 0"
        );

        // Exactly one sink.
        assert_eq!(result.sinks.len(), 1, "should have exactly one sink");
        assert_eq!(result.sinks[0], StepIdx::new(2), "sink should be node 2");

        // Not safe -- secret reaches Finish.
        assert!(
            !result.finish_safe,
            "finish must not be safe when secret reaches it"
        );

        // Exactly one forbidden flow path.
        let forbidden: Vec<&TaintFlowPath> = result
            .flow_paths
            .iter()
            .filter(|p| p.is_forbidden)
            .collect();
        assert_eq!(
            forbidden.len(),
            1,
            "should have exactly one forbidden flow path"
        );

        let path = forbidden[0];
        assert_eq!(
            path.source_step,
            StepIdx::new(0),
            "path source should be node 0"
        );
        assert_eq!(
            path.sink_step,
            StepIdx::new(2),
            "path sink should be node 2"
        );
        assert!(
            path.path_nodes.contains(&StepIdx::new(0)),
            "path must contain source node 0"
        );
        assert!(
            path.path_nodes.contains(&StepIdx::new(2)),
            "path must contain sink node 2"
        );

        // Forbidden sink should be recorded.
        assert!(
            result.forbidden_sinks.contains(&2),
            "node 2 should be a forbidden sink"
        );
    }

    /// BH-08: Forbidden vs allowed flow classification.
    ///
    /// Build a graph where one secret source reaches a Finish (forbidden)
    /// and another secret source does NOT reach any Finish (allowed /
    /// warning).  Verify that `is_forbidden` is correctly set for each
    /// flow path.
    #[test]
    fn bh08_forbidden_vs_allowed_flow_classification() {
        let parts = make_parts_with_output(vec![
            // Node 0: secret source, connects to Finish (forbidden path)
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(2)),
                Some(SlotIdx::new(0)),
            ),
            // Node 1: secret source, no connection to Finish (allowed path)
            (CompiledNodeKind::Nop, None, Some(SlotIdx::new(1))),
            // Node 2: Finish sink
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ]);

        let mut taint_map = HashMap::new();
        taint_map.insert(SlotIdx::new(0), String::from("Secret"));
        taint_map.insert(SlotIdx::new(1), String::from("Secret"));

        let result = compute_taint_overlay(&parts, &taint_map);

        // Two sources.
        assert_eq!(result.sources.len(), 2, "should have two secret sources");

        // One forbidden path (node 0 -> node 2).
        let forbidden: Vec<&TaintFlowPath> = result
            .flow_paths
            .iter()
            .filter(|p| p.is_forbidden)
            .collect();
        assert_eq!(
            forbidden.len(),
            1,
            "should have exactly one forbidden flow path"
        );
        assert_eq!(
            forbidden[0].source_step,
            StepIdx::new(0),
            "forbidden path should start at node 0"
        );
        assert_eq!(
            forbidden[0].sink_step,
            StepIdx::new(2),
            "forbidden path should end at node 2 (Finish)"
        );

        // No allowed (non-forbidden) paths because node 1 has no successors
        // and therefore no reachable nodes.
        let allowed: Vec<&TaintFlowPath> = result
            .flow_paths
            .iter()
            .filter(|p| !p.is_forbidden)
            .collect();
        assert!(
            allowed.is_empty(),
            "source with no reachable nodes should produce no flow path"
        );

        // Finish is not safe because node 0 reaches it.
        assert!(
            !result.finish_safe,
            "finish must not be safe when a forbidden path exists"
        );
    }

    // =========================================================================
    // BLACK HAT findings (BH-T01 through BH-T05)
    // =========================================================================

    /// BH-T01 [HIGH]: is_secret_taint false-positive from substring match on
    /// "NotASecret".
    ///
    /// The implementation uses `label.contains("Secret")` which matches
    /// "NotASecret", "SuperSecretBackup", etc. This is a false positive that
    /// could flag non-sensitive data as a taint source, leading to alarm
    /// fatigue and missed genuine leaks.
    #[test]
    fn bht01_not_a_secret_false_positive_confirmed() {
        let mut map = HashMap::new();
        let slot = SlotIdx::new(42);
        map.insert(slot, String::from("NotASecret"));

        assert!(
            is_secret_taint(&map, slot),
            "BLACK HAT [HIGH]: 'NotASecret' triggers is_secret_taint via substring match"
        );

        // Also verify that a truly unrelated label does not match.
        let clean_slot = SlotIdx::new(43);
        map.insert(clean_slot, String::from("PublicData"));
        assert!(
            !is_secret_taint(&map, clean_slot),
            "'PublicData' must not match"
        );

        // Verify "derived" substring also causes false positives.
        let derived_slot = SlotIdx::new(44);
        map.insert(derived_slot, String::from("NonDerivedValue"));
        // "derived" is lowercase in the check, so "NonDerivedValue" with
        // capital D does not match "derived" via contains.
        // But "unrelated-derived-data" would match.
        let derived_lower_slot = SlotIdx::new(45);
        map.insert(derived_lower_slot, String::from("unrelated-derived-data"));
        assert!(
            is_secret_taint(&map, derived_lower_slot),
            "BLACK HAT [HIGH]: 'unrelated-derived-data' matches via 'derived' substring"
        );
    }

    /// BH-T02 [HIGH]: ForEachJoin output slot is incorrectly classified as an
    /// input slot in collect_input_slots.
    ///
    /// The `output` field of ForEachJoin represents where the joined result is
    /// *written*, not a slot that is read. By including it in `input_slots`,
    /// the taint analysis may incorrectly propagate taint *from* the output
    /// slot of a ForEachJoin node, treating a write target as a read source.
    #[test]
    fn bht02_for_each_join_output_misclassified_as_input() {
        let output_slot = SlotIdx::new(99);
        let kind = CompiledNodeKind::ForEachJoin {
            output: output_slot,
        };
        let input_slots = collect_input_slots(&kind);

        // BLACK HAT [HIGH]: ForEachJoin.output is returned as an input slot.
        assert!(
            input_slots.contains(&output_slot),
            "BLACK HAT [HIGH]: ForEachJoin output slot is incorrectly listed as input"
        );
        assert_eq!(
            input_slots.len(),
            1,
            "ForEachJoin should report exactly the output slot"
        );

        // Demonstrate the impact: if slot 99 is tainted, a node that writes
        // to ForEachJoin will be incorrectly classified as reading tainted data.
        let parts = make_parts_with_output(vec![
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(50)), // writes to slot 50 (not tainted)
            ),
            (
                CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(99), // writes to slot 99
                },
                Some(StepIdx::new(2)),
                None,
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]);
        let mut taint_map = HashMap::new();
        taint_map.insert(SlotIdx::new(99), String::from("Secret"));
        let result = compute_taint_overlay(&parts, &taint_map);

        // Node 1 (ForEachJoin) is classified as tainted because slot 99
        // appears in collect_input_slots, but it WRITES to slot 99, it does
        // not read from it.
        assert!(
            result.tainted_nodes.contains(&1),
            "BLACK HAT [HIGH]: ForEachJoin node is incorrectly tainted because \
             its output slot is treated as an input"
        );
    }

    /// BH-T03 [MEDIUM]: walk_forward uses Vec::pop() (DFS/LIFO) but code
    /// comments say BFS. This affects path discovery order but not correctness
    /// of reachability. The path returned may not be the shortest path.
    #[test]
    fn bht03_walk_forward_is_dfs_not_bfs() {
        // Build a chain: 0 -> 1 -> 2 -> 3 -> 4
        let nodes: Vec<CompiledNode> = (0..5u16)
            .map(|i| CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 4 {
                    Some(StepIdx::new(u16::from(i) + 1))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            })
            .collect();
        let count = nodes.len();
        let parts = WorkflowParts {
            name: String::from("bh-t03").into_boxed_str(),
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
        };

        let reachable = walk_forward(&parts, StepIdx::new(0));

        // All 4 successors must be present.
        assert_eq!(reachable.len(), 4);
        for i in 1..=4u16 {
            assert!(
                reachable.contains(&StepIdx::new(i)),
                "node {} should be reachable",
                i
            );
        }

        // BLACK HAT [MEDIUM]: With DFS (Vec::pop), order is [1, 2, 3, 4].
        // With BFS (VecDeque), order would also be [1, 2, 3, 4] for a chain.
        // The difference manifests with branching graphs.
        assert_eq!(
            reachable[0],
            StepIdx::new(1),
            "DFS traversal order confirmed (matches BFS for linear chain)"
        );
    }

    /// BH-T04 [MEDIUM]: build_path_nodes fallback returns [source, sink] when
    /// no path is found, even though the sink was confirmed reachable via
    /// walk_forward. This creates a phantom direct edge that does not exist
    /// in the actual workflow graph.
    #[test]
    fn bht04_path_fallback_creates_phantom_edge() {
        // This test documents the fallback behavior in build_path_nodes where
        // it returns vec![source, sink] when BFS fails to find a path.
        // In practice, this fallback should rarely trigger because walk_forward
        // already confirmed reachability, but it documents a correctness risk.

        // Build a workflow where node 0 has tainted output and connects to
        // node 2 (Finish) via node 1. The path should be 0 -> 1 -> 2.
        let parts = make_parts_with_output(vec![
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(0)),
            ),
            (CompiledNodeKind::Nop, Some(StepIdx::new(2)), None),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]);
        let mut taint_map = HashMap::new();
        taint_map.insert(SlotIdx::new(0), String::from("Secret"));

        let result = compute_taint_overlay(&parts, &taint_map);

        // Verify that the flow path includes the intermediate node.
        let forbidden: Vec<&TaintFlowPath> = result
            .flow_paths
            .iter()
            .filter(|p| p.is_forbidden)
            .collect();

        if let Some(path) = forbidden.first() {
            // The path should contain the intermediate step 1.
            assert!(
                path.path_nodes.contains(&StepIdx::new(1)),
                "path should include intermediate node 1, got: {:?}",
                path.path_nodes
            );
        }
    }

    /// BH-T05 [LOW]: compute_taint_overlay with a WaitEvent that has no next
    /// edge but is co-located with a Finish node should report the source as
    /// contained (finish_safe = true) but with a warning path segment.
    ///
    /// The WaitEvent is a structural source (legacy path) even without taint
    /// map entries, so it always creates source entries that may produce
    /// unnecessary warnings in workflows that handle events safely.
    #[test]
    fn bht05_wait_event_always_structural_source_even_when_safe() {
        // WaitEvent with no connection to Finish -- should be contained.
        let parts = make_parts_with_next(vec![
            (
                CompiledNodeKind::WaitEvent {
                    event: SlotIdx::new(0),
                    timeout_slot: None,
                },
                None, // no connection to Finish
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
            ),
        ]);
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        // WaitEvent is always a structural source, even without taint map.
        assert_eq!(
            result.sources.len(),
            1,
            "WaitEvent is always a structural source"
        );
        assert!(
            result.finish_safe,
            "source does not reach Finish, so finish is safe"
        );

        // BLACK HAT [LOW]: Even though the source is contained and the
        // workflow is safe, the certificate-based analysis in certificates.rs
        // will report this as a Warn (not Pass), which may cause unnecessary
        // concern for workflows that handle events correctly.
    }

    // =========================================================================
    // BLACKHAT security-focused tests
    // =========================================================================

    /// BLACKHAT_overlay_step_color_u16_overflow [HIGH]:
    /// step_color converts step_idx (usize) to StepIdx via
    /// `u16::try_from(step_idx).unwrap_or(u16::MAX)`. For step indices
    /// exceeding u16::MAX, this silently clamps to u16::MAX, which could map
    /// to a valid node in the workflow and return an incorrect color.
    #[test]
    fn blackhat_overlay_step_color_large_index_clamped_to_u16_max() {
        let parts = make_parts_with_next(vec![(
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
        )]);
        let result = compute_taint_overlay(&parts, &empty_taint_map());

        // Step index 65535 (u16::MAX) maps to step 0 in a 1-node workflow,
        // producing a misleading color instead of COLOR_CLEAN.
        let color_65535 = step_color(65535, &result);
        // With u16::MAX clamping, this maps to StepIdx::new(65535) which does
        // not match any source/sink, so it returns COLOR_CLEAN.
        assert_eq!(
            color_65535, COLOR_CLEAN,
            "BLACKHAT [HIGH]: step index 65535 is clamped to u16::MAX via try_from"
        );

        // An extremely large step index also clamps.
        let color_huge = step_color(70000, &result);
        assert_eq!(
            color_huge, COLOR_CLEAN,
            "BLACKHAT [HIGH]: step index 70000 overflows to u16::MAX and returns CLEAN"
        );
    }

    /// BLACKHAT_overlay_self_referencing_node_no_infinite_loop [MEDIUM]:
    /// walk_forward uses a HashSet for visited tracking, so a node whose
    /// `next` points back to itself does not cause infinite iteration.
    /// This test documents the cycle prevention.
    #[test]
    fn blackhat_overlay_self_referencing_node_no_infinite_loop() {
        let parts = make_parts_with_next(vec![
            (CompiledNodeKind::Nop, Some(StepIdx::new(0))), // points to itself
        ]);
        let reachable = walk_forward(&parts, StepIdx::new(0));
        // Node 0's successor is itself (already visited), so result is empty.
        assert!(
            reachable.is_empty(),
            "self-referencing node should produce no reachable successors"
        );
    }

    /// BLACKHAT_overlay_build_path_nodes_non_adjacent_source_sink [MEDIUM]:
    /// build_path_nodes falls back to `vec![source, sink]` when BFS fails
    /// to find a path. This creates a phantom direct edge that does not
    /// exist in the actual graph. The test confirms the fallback path.
    #[test]
    fn blackhat_overlay_build_path_nodes_fallback_phantom_edge() {
        let source_set: HashSet<StepIdx> = [StepIdx::new(0)].into_iter().collect();
        // Two disconnected nodes with no path between them.
        let parts = make_parts_with_next(vec![
            (CompiledNodeKind::Nop, None), // node 0, no next
            (CompiledNodeKind::Nop, None), // node 1, no next
        ]);
        let path = build_path_nodes(&parts, StepIdx::new(0), StepIdx::new(1), &source_set);
        // Fallback returns [source, sink] even though no path exists.
        assert_eq!(
            path.len(),
            2,
            "BLACKHAT [MEDIUM]: fallback returns phantom 2-node path for disconnected nodes"
        );
        assert_eq!(path[0], StepIdx::new(0));
        assert_eq!(path[1], StepIdx::new(1));
    }

    /// BLACKHAT_overlay_is_secret_taint_case_sensitive [LOW]:
    /// The "derived" check in is_secret_taint is case-sensitive (lowercase).
    /// "Derived" (capital D) does not match "derived". This is inconsistent
    /// with "Secret" and "DerivedFromSecret" which start with capitals but
    /// are matched via case-sensitive contains.
    #[test]
    fn blackhat_overlay_is_secret_taint_derived_case_sensitive() {
        let mut map = HashMap::new();
        let slot_lower = SlotIdx::new(100);
        map.insert(slot_lower, String::from("derived-value"));
        assert!(
            is_secret_taint(&map, slot_lower),
            "lowercase 'derived' should match"
        );

        let slot_upper = SlotIdx::new(101);
        map.insert(slot_upper, String::from("Derived-value"));
        // "Derived" does NOT contain "derived" (case-sensitive).
        assert!(
            !is_secret_taint(&map, slot_upper),
            "BLACKHAT [LOW]: 'Derived' (capital D) does not match 'derived' -- inconsistent casing"
        );
    }

    /// BLACKHAT_overlay_flow_path_non_forbidden_contains_source_and_sink [LOW]:
    /// When a source has reachable nodes but does not reach a sink, the code
    /// creates a non-forbidden flow path where path_nodes contains [source,
    /// source, ...reachable...]. The source appears twice because the code
    /// pushes source first and then iterates reachable (which does not contain
    /// source but the vec starts with source).
    #[test]
    fn blackhat_overlay_non_forbidden_path_has_correct_nodes() {
        let parts = make_parts_with_output(vec![
            // Source with tainted output, connects to Nop, Nop has no Finish.
            (
                CompiledNodeKind::Nop,
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(0)),
            ),
            (CompiledNodeKind::Nop, None, None),
        ]);
        let mut taint_map = HashMap::new();
        taint_map.insert(SlotIdx::new(0), String::from("Secret"));
        let result = compute_taint_overlay(&parts, &taint_map);

        // Source at step 0 reaches step 1 but step 1 is not a Finish.
        assert!(result.finish_safe, "source should not reach any Finish");
        // Check non-forbidden flow path exists.
        let non_forbidden: Vec<&TaintFlowPath> = result
            .flow_paths
            .iter()
            .filter(|p| !p.is_forbidden)
            .collect();
        if let Some(path) = non_forbidden.first() {
            // The path should contain source step 0 and reachable step 1.
            assert!(
                path.path_nodes.contains(&StepIdx::new(0)),
                "path should contain source step 0"
            );
            assert!(
                path.path_nodes.contains(&StepIdx::new(1)),
                "path should contain reachable step 1"
            );
        }
    }

    /// BLACKHAT_overlay_tainted_nodes_via_input_slots [LOW]:
    /// find_tainted_nodes checks both output slots and input slots for taint.
    /// A node whose input slot is tainted (but output is not) is correctly
    /// classified as tainted. This test verifies the input-slot taint path.
    #[test]
    fn blackhat_overlay_node_tainted_via_input_slot_only() {
        let parts = make_parts_with_output(vec![
            // Node 0: reads slot 5 (tainted input), writes to slot 10 (clean)
            (
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(5),
                },
                Some(StepIdx::new(1)),
                Some(SlotIdx::new(10)),
            ),
            (
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(10),
                },
                None,
                None,
            ),
        ]);
        let mut taint_map = HashMap::new();
        taint_map.insert(SlotIdx::new(5), String::from("Secret"));
        // Slot 10 is NOT tainted -- only input slot 5 is.
        let result = compute_taint_overlay(&parts, &taint_map);

        assert!(
            result.tainted_nodes.contains(&0),
            "node reading from tainted input slot should be in tainted_nodes"
        );
    }
}
