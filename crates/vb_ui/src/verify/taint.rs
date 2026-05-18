#![forbid(unsafe_code)]
//! Taint tracking data structures for the verification view.
//!
//! Provides slot-level taint classification and propagation tracking used by
//! the verification screen to visualise how sensitive data flows through a
//! compiled workflow. The types here complement the overlay rendering in
//! `taint_overlay.rs` by offering structured, queryable taint graphs.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::theme::colors;

// ---------------------------------------------------------------------------
// TaintKind -- classification of sensitive data
// ---------------------------------------------------------------------------

/// Classification of the kind of sensitive data carried by a slot.
///
/// Severity ordering (highest to lowest): Secret > Pii > Financial >
/// Authentication > Custom.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TaintKind {
    /// Cryptographic secrets, API keys, passwords.
    Secret,
    /// Personally identifiable information (names, emails, addresses).
    Pii,
    /// Financial data (credit card numbers, bank accounts).
    Financial,
    /// Authentication tokens, session IDs, OAuth credentials.
    Authentication,
    /// User-defined taint category.
    Custom(String),
}

impl TaintKind {
    /// Returns the cyberpunk palette colour for this taint kind.
    ///
    /// - Secret: neon magenta (`#ff00ff`)
    /// - Pii: neon pink (`#ff2d7b`)
    /// - Financial: neon orange (`#ff6b00`)
    /// - Authentication: neon purple (`#b14dff`)
    /// - Custom: neon cyan (`#00f5ff`)
    #[must_use]
    pub fn color(&self) -> [f32; 4] {
        match self {
            Self::Secret => colors::neon::MAGENTA,
            Self::Pii => colors::neon::PINK,
            Self::Financial => colors::neon::ORANGE,
            Self::Authentication => colors::neon::PURPLE,
            Self::Custom(_) => colors::neon::CYAN,
        }
    }

    /// Numeric severity rank (higher is more dangerous).
    ///
    /// Secret = 4, Pii = 3, Financial = 2, Authentication = 1, Custom = 0.
    #[must_use]
    pub fn severity_rank(&self) -> u8 {
        match self {
            Self::Secret => 4,
            Self::Pii => 3,
            Self::Financial => 2,
            Self::Authentication => 1,
            Self::Custom(_) => 0,
        }
    }
}

impl std::fmt::Display for TaintKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Secret => write!(f, "Secret"),
            Self::Pii => write!(f, "Pii"),
            Self::Financial => write!(f, "Financial"),
            Self::Authentication => write!(f, "Authentication"),
            Self::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

// ---------------------------------------------------------------------------
// TaintSource -- origin of tainted data
// ---------------------------------------------------------------------------

/// Describes where a taint enters the workflow graph.
///
/// A source is a specific slot at a specific step that introduces data
/// of a given taint kind into the workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaintSource {
    /// The slot index where the taint originates.
    pub slot: u32,
    /// The step index where the taint originates.
    pub step: u16,
    /// The kind of sensitive data introduced.
    pub kind: TaintKind,
}

// ---------------------------------------------------------------------------
// TaintPropagation -- flow of taint from one slot to another
// ---------------------------------------------------------------------------

/// Describes taint flowing from one slot to another via a workflow step.
///
/// Represents a single directed edge in the taint propagation graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaintPropagation {
    /// The source slot that carries taint.
    pub from_slot: u32,
    /// The destination slot that receives taint.
    pub to_slot: u32,
    /// The step that transfers taint from `from_slot` to `to_slot`.
    pub via_step: u16,
}

// ---------------------------------------------------------------------------
// TaintGraph -- queryable taint propagation graph
// ---------------------------------------------------------------------------

/// A directed graph of taint sources and propagation edges.
///
/// Supports queries such as "which slots are tainted?" and "what is the
/// propagation path to a given slot?"
#[derive(Debug, Clone, Default)]
pub struct TaintGraph {
    /// All taint sources (origins).
    sources: Vec<TaintSource>,
    /// All propagation edges (directed).
    propagations: Vec<TaintPropagation>,
}

impl TaintGraph {
    /// Creates a new empty taint graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            propagations: Vec::new(),
        }
    }

    /// Adds a taint source to the graph.
    pub fn add_source(&mut self, source: TaintSource) {
        self.sources.push(source);
    }

    /// Adds a taint propagation edge to the graph.
    pub fn add_propagation(&mut self, prop: TaintPropagation) {
        self.propagations.push(prop);
    }

    /// Returns a reference to the taint sources.
    #[must_use]
    pub fn sources(&self) -> &[TaintSource] {
        &self.sources
    }

    /// Returns a reference to the taint propagations.
    #[must_use]
    pub fn propagations(&self) -> &[TaintPropagation] {
        &self.propagations
    }

    /// Returns all slot indices that are tainted.
    ///
    /// This includes both source slots and any slot reachable via propagation
    /// edges. Duplicates are removed and the result is sorted.
    #[must_use]
    pub fn tainted_slots(&self) -> Vec<u32> {
        let mut slots: HashSet<u32> = HashSet::new();

        for source in &self.sources {
            slots.insert(source.slot);
        }

        // Build forward adjacency for BFS: from_slot -> list of to_slots.
        let mut forward: HashMap<u32, Vec<u32>> = HashMap::new();
        for prop in &self.propagations {
            forward
                .entry(prop.from_slot)
                .or_default()
                .push(prop.to_slot);
        }

        // BFS from source slots through the propagation graph.
        let mut queue: VecDeque<u32> = slots.iter().copied().collect();
        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = forward.get(&current) {
                for &neighbor in neighbors {
                    if slots.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        let mut result: Vec<u32> = slots.into_iter().collect();
        result.sort_unstable();
        result
    }

    /// Returns the propagation chain leading to a given slot.
    ///
    /// Traces backwards from `slot` through propagation edges to find the
    /// shortest chain of propagations from any source slot to `slot`.
    /// Returns an empty vec if `slot` is not tainted or is only a source.
    #[must_use]
    pub fn propagation_path_to(&self, slot: u32) -> Vec<TaintPropagation> {
        let source_slots: HashSet<u32> = self.sources.iter().map(|s| s.slot).collect();

        // Build reverse adjacency: to_slot -> list of propagations leading to it.
        let mut incoming: HashMap<u32, Vec<&TaintPropagation>> = HashMap::new();
        for prop in &self.propagations {
            incoming.entry(prop.to_slot).or_default().push(prop);
        }

        // If the slot is a source and has no incoming edges, return empty.
        if source_slots.contains(&slot) && !incoming.contains_key(&slot) {
            return Vec::new();
        }

        // If the slot is not tainted at all, return empty.
        if !source_slots.contains(&slot) && !self.propagations.iter().any(|p| p.to_slot == slot) {
            return Vec::new();
        }

        // BFS backwards from `slot` to find shortest path to any source.
        let mut parent: HashMap<u32, &TaintPropagation> = HashMap::new();
        let mut visited: HashSet<u32> = HashSet::new();
        visited.insert(slot);
        let mut queue: VecDeque<u32> = VecDeque::new();
        queue.push_back(slot);

        let mut found_source: Option<u32> = None;

        while let Some(current) = queue.pop_front() {
            if source_slots.contains(&current) && current != slot {
                found_source = Some(current);
                break;
            }

            if let Some(preds) = incoming.get(&current) {
                for prop in preds {
                    if visited.insert(prop.from_slot) {
                        // Map predecessor node -> propagation for reconstruction
                        parent.insert(prop.from_slot, *prop);
                        queue.push_back(prop.from_slot);
                    }
                }
            }
        }

        // If the target slot itself is a source, no propagation path needed.
        if source_slots.contains(&slot) {
            return Vec::new();
        }

        // If we didn't find a source, the slot may still be reachable but
        // we cannot trace a complete path. Return empty.
        let Some(source_slot) = found_source else {
            return Vec::new();
        };

        // Reconstruct path from source to slot by following parent entries.
        // Each parent entry maps a node to the propagation edge that discovered it,
        // so walking from source_slot forward via prop.to_slot yields the shortest path.
        let mut path: Vec<TaintPropagation> = Vec::new();
        let mut current = source_slot;
        while let Some(&prop) = parent.get(&current) {
            path.push(prop.clone());
            current = prop.to_slot;
        }
        path
    }

    /// Returns the most dangerous taint kind present in the graph.
    ///
    /// Severity ordering: Secret > Pii > Financial > Authentication > Custom.
    /// Returns `None` if the graph has no sources.
    #[must_use]
    pub fn highest_severity(&self) -> Option<TaintKind> {
        self.sources
            .iter()
            .map(|s| &s.kind)
            .max_by_key(|k| k.severity_rank())
            .cloned()
    }
}

// ---------------------------------------------------------------------------
// Legacy function (preserved for backwards compatibility)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // TaintKind tests
    // ========================================================================

    #[test]
    fn test_taint_kind_color_secret_is_magenta() {
        assert_eq!(TaintKind::Secret.color(), colors::neon::MAGENTA);
    }

    #[test]
    fn test_taint_kind_color_pii_is_pink() {
        assert_eq!(TaintKind::Pii.color(), colors::neon::PINK);
    }

    #[test]
    fn test_taint_kind_color_financial_is_orange() {
        assert_eq!(TaintKind::Financial.color(), colors::neon::ORANGE);
    }

    #[test]
    fn test_taint_kind_color_authentication_is_purple() {
        assert_eq!(TaintKind::Authentication.color(), colors::neon::PURPLE);
    }

    #[test]
    fn test_taint_kind_color_custom_is_cyan() {
        assert_eq!(
            TaintKind::Custom(String::from("user-data")).color(),
            colors::neon::CYAN
        );
    }

    #[test]
    fn test_taint_kind_severity_ordering() {
        assert!(TaintKind::Secret.severity_rank() > TaintKind::Pii.severity_rank());
        assert!(TaintKind::Pii.severity_rank() > TaintKind::Financial.severity_rank());
        assert!(TaintKind::Financial.severity_rank() > TaintKind::Authentication.severity_rank());
        assert!(
            TaintKind::Authentication.severity_rank()
                > TaintKind::Custom(String::new()).severity_rank()
        );
    }

    #[test]
    fn test_taint_kind_display() {
        assert_eq!(format!("{}", TaintKind::Secret), "Secret");
        assert_eq!(format!("{}", TaintKind::Pii), "Pii");
        assert_eq!(format!("{}", TaintKind::Financial), "Financial");
        assert_eq!(format!("{}", TaintKind::Authentication), "Authentication");
        assert_eq!(
            format!("{}", TaintKind::Custom(String::from("token"))),
            "Custom(token)"
        );
    }

    // ========================================================================
    // TaintGraph -- add_source / add_propagation
    // ========================================================================

    #[test]
    fn test_graph_add_source_and_access() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 1,
            kind: TaintKind::Secret,
        });
        assert_eq!(graph.sources().len(), 1);
        assert_eq!(graph.sources()[0].slot, 0);
        assert_eq!(graph.sources()[0].step, 1);
        assert_eq!(graph.sources()[0].kind, TaintKind::Secret);
    }

    #[test]
    fn test_graph_add_propagation_and_access() {
        let mut graph = TaintGraph::new();
        graph.add_propagation(TaintPropagation {
            from_slot: 0,
            to_slot: 1,
            via_step: 2,
        });
        assert_eq!(graph.propagations().len(), 1);
        assert_eq!(graph.propagations()[0].from_slot, 0);
        assert_eq!(graph.propagations()[0].to_slot, 1);
        assert_eq!(graph.propagations()[0].via_step, 2);
    }

    // ========================================================================
    // TaintGraph -- tainted_slots
    // ========================================================================

    #[test]
    fn test_tainted_slots_empty_graph() {
        let graph = TaintGraph::new();
        assert!(graph.tainted_slots().is_empty());
    }

    #[test]
    fn test_tainted_slots_only_sources() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 5,
            step: 0,
            kind: TaintKind::Pii,
        });
        graph.add_source(TaintSource {
            slot: 10,
            step: 1,
            kind: TaintKind::Secret,
        });
        let slots = graph.tainted_slots();
        assert_eq!(slots, vec![5u32, 10u32]);
    }

    #[test]
    fn test_tainted_slots_includes_propagated() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Secret,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 0,
            to_slot: 1,
            via_step: 1,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 1,
            to_slot: 3,
            via_step: 2,
        });
        let slots = graph.tainted_slots();
        assert_eq!(slots, vec![0u32, 1u32, 3u32]);
    }

    // ========================================================================
    // TaintGraph -- propagation_path_to
    // ========================================================================

    #[test]
    fn test_propagation_path_to_source_slot_returns_empty() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Secret,
        });
        assert!(graph.propagation_path_to(0).is_empty());
    }

    #[test]
    fn test_propagation_path_to_unrelated_slot_returns_empty() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Secret,
        });
        assert!(graph.propagation_path_to(99).is_empty());
    }

    #[test]
    fn test_propagation_path_to_direct_propagation() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Secret,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 0,
            to_slot: 2,
            via_step: 1,
        });
        let path = graph.propagation_path_to(2);
        assert_eq!(path.len(), 1);
        assert_eq!(path[0].from_slot, 0);
        assert_eq!(path[0].to_slot, 2);
        assert_eq!(path[0].via_step, 1);
    }

    #[test]
    fn test_propagation_path_to_chain() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Secret,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 0,
            to_slot: 1,
            via_step: 1,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 1,
            to_slot: 3,
            via_step: 2,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 3,
            to_slot: 5,
            via_step: 3,
        });
        let path = graph.propagation_path_to(5);
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].from_slot, 0);
        assert_eq!(path[0].to_slot, 1);
        assert_eq!(path[1].from_slot, 1);
        assert_eq!(path[1].to_slot, 3);
        assert_eq!(path[2].from_slot, 3);
        assert_eq!(path[2].to_slot, 5);
    }

    // ========================================================================
    // TaintGraph -- highest_severity
    // ========================================================================

    #[test]
    fn test_highest_severity_empty_graph() {
        let graph = TaintGraph::new();
        assert!(graph.highest_severity().is_none());
    }

    #[test]
    fn test_highest_severity_single_source() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Financial,
        });
        assert_eq!(graph.highest_severity(), Some(TaintKind::Financial));
    }

    #[test]
    fn test_highest_severity_picks_most_dangerous() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Custom(String::from("log")),
        });
        graph.add_source(TaintSource {
            slot: 1,
            step: 1,
            kind: TaintKind::Authentication,
        });
        graph.add_source(TaintSource {
            slot: 2,
            step: 2,
            kind: TaintKind::Pii,
        });
        graph.add_source(TaintSource {
            slot: 3,
            step: 3,
            kind: TaintKind::Secret,
        });
        assert_eq!(graph.highest_severity(), Some(TaintKind::Secret));
    }

    #[test]
    fn test_highest_severity_no_secret_picks_pii() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Financial,
        });
        graph.add_source(TaintSource {
            slot: 1,
            step: 1,
            kind: TaintKind::Pii,
        });
        assert_eq!(graph.highest_severity(), Some(TaintKind::Pii));
    }

    // ========================================================================
    // Legacy find_secret_sources tests
    // ========================================================================

    use vb_core::ids::SlotIdx;
    use vb_core::ids::WorkflowDigest;
    use vb_core::workflow::{CompiledNode, ResourceContract};

    fn make_parts(kinds: Vec<CompiledNodeKind>) -> WorkflowParts {
        let nodes: Vec<CompiledNode> = kinds
            .into_iter()
            .enumerate()
            .map(|(i, kind)| CompiledNode {
                id: StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX)),
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
            step_names: (0..count)
                .map(|_| Box::<str>::from(""))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
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

    // ========================================================================
    // TaintGraph -- transitive / diamond propagation
    // ========================================================================

    #[test]
    fn test_tainted_slots_diamond_propagation() {
        // 0 -> 1, 0 -> 2, 1 -> 3, 2 -> 3 (diamond merge)
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Secret,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 0,
            to_slot: 1,
            via_step: 1,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 0,
            to_slot: 2,
            via_step: 2,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 1,
            to_slot: 3,
            via_step: 3,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 2,
            to_slot: 3,
            via_step: 4,
        });
        let slots = graph.tainted_slots();
        assert_eq!(slots, vec![0u32, 1u32, 2u32, 3u32]);
    }

    #[test]
    fn test_propagation_path_to_diamond_picks_shortest() {
        // 0 -> 1 (via step 1), 1 -> 3 (via step 3)
        // 0 -> 2 (via step 2), 2 -> 3 (via step 4)
        // Path to 3 should be one of the two shortest chains.
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Secret,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 0,
            to_slot: 1,
            via_step: 1,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 0,
            to_slot: 2,
            via_step: 2,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 1,
            to_slot: 3,
            via_step: 3,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 2,
            to_slot: 3,
            via_step: 4,
        });
        let path = graph.propagation_path_to(3);
        assert_eq!(path.len(), 2);
        // Path should start at source slot 0 and end at slot 3.
        assert_eq!(path[0].from_slot, 0);
        assert_eq!(path[1].to_slot, 3);
    }

    #[test]
    fn test_tainted_slots_with_orphan_propagation() {
        // Propagation edge whose source is not tainted -- should not leak.
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 5,
            step: 0,
            kind: TaintKind::Secret,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 0, // not tainted
            to_slot: 1,
            via_step: 1,
        });
        let slots = graph.tainted_slots();
        assert_eq!(slots, vec![5u32]);
    }

    // =========================================================================
    // BLACKHAT security-focused tests
    // =========================================================================

    /// BLACKHAT_taint_propagation_cycle_no_infinite_loop [CONFIRMED-SAFE]:
    /// A propagation cycle (A -> B -> A) should not cause infinite iteration
    /// because BFS uses a HashSet for visited tracking.
    #[test]
    fn blackhat_taint_propagation_cycle_no_infinite_loop() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Secret,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 0,
            to_slot: 1,
            via_step: 1,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 1,
            to_slot: 0, // cycle back to source
            via_step: 2,
        });
        let slots = graph.tainted_slots();
        assert_eq!(
            slots,
            vec![0u32, 1u32],
            "cycle should terminate and include both slots"
        );
    }

    /// BLACKHAT_taint_propagation_path_to_nonexistent_slot [LOW]:
    /// propagation_path_to for a slot that exists as a to_slot target but
    /// has no path back to a source returns an empty vec. The test documents
    /// this edge case.
    #[test]
    fn blackhat_taint_propagation_path_to_unreachable_target() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Secret,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 5, // not tainted -- no source at slot 5
            to_slot: 10,
            via_step: 1,
        });
        let path = graph.propagation_path_to(10);
        assert!(
            path.is_empty(),
            "slot 10 is not reachable from any source, path should be empty"
        );
    }

    /// BLACKHAT_taint_custom_kind_has_lowest_severity [CONFIRMED-SAFE]:
    /// Custom taint kinds always have severity 0, lower than all built-in
    /// kinds. This means they never dominate in highest_severity.
    #[test]
    fn blackhat_taint_custom_kind_always_lowest() {
        let custom = TaintKind::Custom(String::from("important"));
        assert_eq!(custom.severity_rank(), 0);
        assert!(
            custom.severity_rank() < TaintKind::Authentication.severity_rank(),
            "Custom must be lower than Authentication (rank 1)"
        );
    }

    /// BLACKHAT_taint_highest_severity_multiple_same_kind [CONFIRMED-SAFE]:
    /// When multiple sources have the same highest severity, highest_severity
    /// returns that kind (the first one found by max_by_key).
    #[test]
    fn blackhat_taint_highest_severity_duplicate_kinds() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Secret,
        });
        graph.add_source(TaintSource {
            slot: 1,
            step: 1,
            kind: TaintKind::Secret,
        });
        graph.add_source(TaintSource {
            slot: 2,
            step: 2,
            kind: TaintKind::Pii,
        });
        assert_eq!(graph.highest_severity(), Some(TaintKind::Secret));
    }

    /// BLACKHAT_taint_propagation_path_to_source_with_no_edges [CONFIRMED-SAFE]:
    /// A source slot with no outgoing propagation edges returns an empty path
    /// from propagation_path_to (it is a source, not a propagated target).
    #[test]
    fn blackhat_taint_source_with_no_edges_empty_path() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 42,
            step: 0,
            kind: TaintKind::Financial,
        });
        let path = graph.propagation_path_to(42);
        assert!(
            path.is_empty(),
            "source slot with no propagation edges should return empty path"
        );
    }

    /// BLACKHAT_taint_propagation_diamond_same_slot_reachable_twice [LOW]:
    /// In a diamond where two different source slots propagate to the same
    /// target slot, propagation_path_to returns the shortest path from one
    /// of the sources. The test documents that only one path is returned.
    #[test]
    fn blackhat_taint_diamond_same_target_from_two_sources() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 0,
            step: 0,
            kind: TaintKind::Secret,
        });
        graph.add_source(TaintSource {
            slot: 10,
            step: 1,
            kind: TaintKind::Pii,
        });
        // Both propagate to slot 5.
        graph.add_propagation(TaintPropagation {
            from_slot: 0,
            to_slot: 5,
            via_step: 2,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 10,
            to_slot: 5,
            via_step: 3,
        });
        let path = graph.propagation_path_to(5);
        // Should return a single-edge path from one of the sources.
        assert_eq!(path.len(), 1);
        // The path should come from one of the two sources.
        assert!(
            path[0].from_slot == 0 || path[0].from_slot == 10,
            "path should originate from slot 0 or slot 10"
        );
    }

    /// BLACKHAT_taint_large_slot_count_still_correct [CONFIRMED-SAFE]:
    /// Tainted slots are sorted and deduplicated regardless of how many
    /// slots are involved. This test uses a larger graph to confirm.
    #[test]
    fn blackhat_taint_large_slot_count_sorted_deduplicated() {
        let mut graph = TaintGraph::new();
        graph.add_source(TaintSource {
            slot: 50,
            step: 0,
            kind: TaintKind::Secret,
        });
        graph.add_source(TaintSource {
            slot: 10,
            step: 1,
            kind: TaintKind::Pii,
        });
        graph.add_source(TaintSource {
            slot: 10, // duplicate source slot
            step: 2,
            kind: TaintKind::Financial,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 50,
            to_slot: 100,
            via_step: 3,
        });
        graph.add_propagation(TaintPropagation {
            from_slot: 100,
            to_slot: 200,
            via_step: 4,
        });
        let slots = graph.tainted_slots();
        assert_eq!(slots, vec![10u32, 50, 100, 200]);
    }
}
