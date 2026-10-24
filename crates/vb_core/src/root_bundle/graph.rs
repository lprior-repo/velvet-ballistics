//! Phase 7 graph validation orchestrator.
//!
//! Coordinates arena layout, DFS traversal, cycle/depth/reachability
//! validation, and taint computation for the owned root bundle constructor.

use crate::ids::SlotIdx;
use crate::root_bundle::errors::{ArenaNode, OwnedRootBundleError};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStoreCheckpointV1;

use self::arena::{Layout, invariant, node_of, reserve};
mod arena;
mod cursor;
mod precheck;
mod walk;

/// DFS coloring for cycle and reachability detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::root_bundle) enum Color {
    /// Not yet visited.
    White,
    /// Currently on the DFS stack (in progress).
    Gray,
    /// Fully processed.
    Black,
}

/// Per-node graph validation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::root_bundle) struct State {
    /// DFS visit color.
    color: Color,
    /// Postorder height (0 for leaves, 1 + max child for containers).
    height: u32,
    /// Cached descendant taint join.
    taint: Taint,
}

impl State {
    /// Fresh unvisited state.
    fn white() -> Self {
        Self {
            color: Color::White,
            height: 0,
            taint: Taint::Clean,
        }
    }
}

/// Graph validation context holding layout and per-node state.
pub(in crate::root_bundle) struct Graph {
    layout: Layout,
    states: Vec<State>,
}

impl Graph {
    /// Get the state for a node.
    pub(in crate::root_bundle) fn state(
        &self,
        node: ArenaNode,
    ) -> Result<State, OwnedRootBundleError> {
        let index = self.layout.index(node)?;
        self.states.get(index).copied().ok_or(invariant())
    }

    /// Set the state for a node.
    pub(in crate::root_bundle) fn set(
        &mut self,
        node: ArenaNode,
        state: State,
    ) -> Result<(), OwnedRootBundleError> {
        let index = self.layout.index(node)?;
        let slot = self.states.get_mut(index).ok_or(invariant())?;
        *slot = state;
        Ok(())
    }

    /// Get the descendant taint for a value.
    pub(in crate::root_bundle) fn descendant_taint(
        &self,
        value: SlotValue,
    ) -> Result<Taint, OwnedRootBundleError> {
        match node_of(value) {
            None => Ok(Taint::Clean),
            Some(node) => {
                let state = self.state(node)?;
                if state.color != Color::Black {
                    return Err(invariant());
                }
                Ok(state.taint)
            }
        }
    }
}

/// Validate the arena graph structure.
pub(super) fn validate(
    cp: &ValueStoreCheckpointV1,
    roots: &[(SlotIdx, SlotValue, Taint)],
) -> Result<Graph, OwnedRootBundleError> {
    precheck::validate(cp)?;

    let layout = Layout::new(cp)?;
    let total = layout.len();
    let states = reserve(total, State::white())?;

    let mut graph = Graph { layout, states };

    for (_, value, _) in roots {
        if let Some(node) = node_of(*value) {
            walk::visit(cp, &mut graph, node)?;
        }
    }

    ensure_reachable(&graph)?;

    Ok(graph)
}

/// Validate every allocation, including structurally valid retired pages.
pub(super) fn validate_runtime(cp: &ValueStoreCheckpointV1) -> Result<Graph, OwnedRootBundleError> {
    precheck::validate(cp)?;
    let layout = Layout::new(cp)?;
    let total = layout.len();
    let states = reserve(total, State::white())?;
    let mut graph = Graph { layout, states };
    for index in 0..total {
        let node = graph.layout.node(index)?;
        walk::visit(cp, &mut graph, node)?;
    }
    Ok(graph)
}

pub use precheck::checked_edge_work;
fn ensure_reachable(graph: &Graph) -> Result<(), OwnedRootBundleError> {
    for (index, state) in graph.states.iter().enumerate() {
        if state.color == Color::White {
            let node = graph.layout.node(index)?;
            return Err(OwnedRootBundleError::GraphUnreachable { node });
        }
    }
    Ok(())
}
