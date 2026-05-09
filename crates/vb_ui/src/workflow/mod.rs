#![forbid(unsafe_code)]
//! Workflow canvas module for the Velvet Ballistics graph editor.
//!
//! Provides the authoring canvas that combines a [`FlowDocument`] with computed
//! layout positions, viewport state, and node selection. The canvas is a pure
//! data structure -- it has no side effects and performs no rendering.
//!
//! Sub-modules:
//! - [`canvas`] -- viewport, selection, focus-jump, edge paths
//! - [`node_mapping`] -- CompiledNodeKind -> visual properties

pub mod canvas;
pub mod node_mapping;

pub use canvas::{
    EdgePath, EdgeType, NodeBadge, ViewportRect, WorkflowCanvas, WorkflowEdge, WorkflowGraph,
    WorkflowNode, build_graph,
};
pub use node_mapping::{NodeCategory, NodeShape, NodeVisual, node_kind_to_visual};
