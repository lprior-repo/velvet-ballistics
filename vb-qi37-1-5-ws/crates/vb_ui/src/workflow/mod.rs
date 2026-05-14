#![forbid(unsafe_code)]
//! Workflow canvas module for the Velvet Ballistics graph editor.
//!
//! Provides the authoring canvas that combines a [`crate::graph_builder::FlowDocument`] with computed
//! layout positions, viewport state, and node selection. The canvas is a pure
//! data structure -- it has no side effects and performs no rendering.
//!
//! Sub-modules:
//! - [`canvas`] -- viewport, selection, focus-jump, edge paths
//! - [`node_mapping`] -- CompiledNodeKind -> visual properties
//! - [`execution_details`] -- Screen 3 Execution Details Graph View

pub mod canvas;
pub mod execution_details;
pub mod node_mapping;

pub use canvas::{
    EdgePath, EdgeType, NodeBadge, ViewportRect, WorkflowCanvas, WorkflowEdge, WorkflowGraph,
    WorkflowNode, build_graph,
};
pub use execution_details::{
    DetailTab, DurabilityDisplayProfile, EventTableRow, ExecutionDetailsError,
    ExecutionDetailsState, ExecutionRunSummary, RunDisplayStatus, RuntimeNodeState, StepDetails,
    build_event_table_rows, build_step_details, execution_details_screen_new, format_elapsed,
    node_runtime_state, step_tab_content,
};
pub use node_mapping::{NodeCategory, NodeShape, NodeVisual, node_kind_to_visual};
