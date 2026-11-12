//! Iterative DFS graph validation for owned root bundle.
//!
//! Validates arena graph structure: cycle detection, depth limits,
//! reachability, and computes postorder heights and taint joins.

use crate::root_bundle::errors::{ArenaNode, OwnedRootBundleError};
use crate::value::{Taint, join_taint};
use crate::value_store::ValueStoreCheckpointV1;

use super::arena::{invariant, node_of};
use super::cursor;
use super::{Color, Graph, State};

mod frame;
use frame::Frame;

/// Visit a node and its descendants, validating graph structure.
pub(in crate::root_bundle) fn visit(
    cp: &ValueStoreCheckpointV1,
    graph: &mut Graph,
    node: ArenaNode,
) -> Result<(), OwnedRootBundleError> {
    if is_root_black(graph, node)? {
        return Ok(());
    }

    if is_leaf(node) {
        return complete_root_leaf(graph, node);
    }

    init_root_stack(graph, node)?;

    let mut stack = Frame::stack(node)?;

    while !stack.is_empty() {
        step(cp, graph, &mut stack)?;
    }

    Ok(())
}

/// Check if the root node is already fully processed.
fn is_root_black(graph: &Graph, node: ArenaNode) -> Result<bool, OwnedRootBundleError> {
    let state = graph.state(node)?;
    Ok(state.color == Color::Black)
}

/// Check if a node is a leaf (symbol or blob).
fn is_leaf(node: ArenaNode) -> bool {
    matches!(node, ArenaNode::Symbol(_) | ArenaNode::Blob(_))
}

/// Complete a leaf root node by marking it black.
fn complete_root_leaf(graph: &mut Graph, node: ArenaNode) -> Result<(), OwnedRootBundleError> {
    mark_leaf_black(graph, node)
}

/// Initialize the root node as gray on the graph.
fn init_root_stack(graph: &mut Graph, node: ArenaNode) -> Result<(), OwnedRootBundleError> {
    graph.set(
        node,
        State {
            color: Color::Gray,
            height: 0,
            taint: Taint::Clean,
        },
    )
}

/// Process one step: either advance an edge or finish a frame.
fn step(
    cp: &ValueStoreCheckpointV1,
    graph: &mut Graph,
    stack: &mut Vec<Frame>,
) -> Result<(), OwnedRootBundleError> {
    let frame = stack.last().ok_or(invariant())?;
    let node = frame.node;
    let cursor = frame.cursor;

    let edge = cursor::edge(cp, node, cursor)?;

    if let Some(edge) = edge {
        process_edge(graph, stack, edge)?;
    } else {
        finish_frame(graph, stack)?;
    }

    Ok(())
}

/// Process a single edge from the current frame.
fn process_edge(
    graph: &mut Graph,
    stack: &mut Vec<Frame>,
    edge: cursor::Edge,
) -> Result<(), OwnedRootBundleError> {
    advance_frame(stack, edge.taint)?;

    if let Some(child_node) = node_of(edge.value) {
        process_child(graph, stack, child_node)?;
    }

    Ok(())
}

/// Advance the current frame's cursor and join edge taint.
fn advance_frame(stack: &mut [Frame], edge_taint: Taint) -> Result<(), OwnedRootBundleError> {
    let frame = stack.last_mut().ok_or(invariant())?;
    frame.taint = join_taint(frame.taint, edge_taint);
    frame.cursor = frame.cursor.checked_add(1).ok_or(invariant())?;
    Ok(())
}

/// Process a child node based on its DFS color.
fn process_child(
    graph: &mut Graph,
    stack: &mut Vec<Frame>,
    child_node: ArenaNode,
) -> Result<(), OwnedRootBundleError> {
    let child_state = graph.state(child_node)?;

    if child_state.color == Color::Black {
        absorb_black_child(stack, child_state)?;
    } else if child_state.color == Color::Gray {
        return Err(OwnedRootBundleError::GraphCycle { node: child_node });
    } else {
        handle_white(graph, stack, child_node)?;
    }

    Ok(())
}

/// Absorb cached height/taint from a black child.
fn absorb_black_child(stack: &mut [Frame], child_state: State) -> Result<(), OwnedRootBundleError> {
    let frame = stack.last_mut().ok_or(invariant())?;
    absorb_black(frame, child_state);
    Ok(())
}

/// Handle a white (unvisited) child node.
fn handle_white(
    graph: &mut Graph,
    stack: &mut Vec<Frame>,
    child_node: ArenaNode,
) -> Result<(), OwnedRootBundleError> {
    if is_leaf(child_node) {
        mark_leaf_black(graph, child_node)?;
    } else {
        push_container(graph, stack, child_node)?;
    }
    Ok(())
}

/// Mark a leaf node (symbol/blob) as black with height 0.
fn mark_leaf_black(graph: &mut Graph, node: ArenaNode) -> Result<(), OwnedRootBundleError> {
    graph.set(
        node,
        State {
            color: Color::Black,
            height: 0,
            taint: Taint::Clean,
        },
    )
}

/// Push a container node onto the DFS stack.
fn push_container(
    graph: &mut Graph,
    stack: &mut Vec<Frame>,
    child_node: ArenaNode,
) -> Result<(), OwnedRootBundleError> {
    if stack.len() >= 64 {
        return Err(OwnedRootBundleError::GraphDepth {
            node: child_node,
            depth: 65,
        });
    }

    init_root_stack(graph, child_node)?;
    stack.push(Frame::new(child_node));
    Ok(())
}

/// Absorb cached height/taint from a black child.
fn absorb_black(frame: &mut Frame, child_state: State) {
    if child_state.height > frame.max_child_height {
        frame.max_child_height = child_state.height;
    }
    frame.taint = join_taint(frame.taint, child_state.taint);
}

/// Finish processing a frame: compute height, mark black, pop.
fn finish_frame(graph: &mut Graph, stack: &mut Vec<Frame>) -> Result<(), OwnedRootBundleError> {
    let frame = stack.pop().ok_or(invariant())?;
    let height = frame.checked_height()?;

    graph.set(frame.node, frame.completed_state(height))?;

    if let Some(parent) = stack.last_mut() {
        absorb_into_parent(parent, height, frame.taint);
    }

    Ok(())
}

/// Absorb completed child's height/taint into parent frame.
fn absorb_into_parent(parent: &mut Frame, height: u32, taint: Taint) {
    if height > parent.max_child_height {
        parent.max_child_height = height;
    }
    parent.taint = join_taint(parent.taint, taint);
}
