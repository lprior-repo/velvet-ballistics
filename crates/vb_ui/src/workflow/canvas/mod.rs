#![forbid(unsafe_code)]
//! Canvas submodules for workflow graph rendering.

mod canvas_area;
mod graph;
mod types;

#[cfg(test)]
mod tests;

pub use canvas_area::WorkflowCanvas;
pub use graph::build_graph;
pub use types::{
    EdgePath, EdgeType, NodeBadge, ViewportRect, WorkflowEdge, WorkflowGraph, WorkflowNode,
    DEFAULT_ZOOM, MAX_ZOOM, MIN_ZOOM,
};
