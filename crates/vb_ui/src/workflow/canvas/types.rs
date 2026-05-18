#![forbid(unsafe_code)]
//! Canvas types: viewport, edge path, graph data model types.

#[cfg(test)]
use std::collections::HashMap;

use std::collections::HashSet;

use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::graph_builder::FlowDocument;
use crate::layout::{LayoutEdge, LayoutNode, LayoutResult};
use crate::workflow::node_mapping::{NodeVisual, node_kind_to_visual};

pub const DEFAULT_ZOOM: f64 = 1.0;
pub const MIN_ZOOM: f64 = 0.1;
pub const MAX_ZOOM: f64 = 5.0;
pub const BEZIER_OFFSET: f64 = 60.0;

#[derive(Debug, Clone, Copy)]
pub struct ViewportRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ViewportRect {
    #[must_use]
    pub fn intersects(&self, other_x: f64, other_y: f64, other_w: f64, other_h: f64) -> bool {
        let self_right = self.x + self.width;
        let self_bottom = self.y + self.height;
        let other_right = other_x + other_w;
        let other_bottom = other_y + other_h;

        let no_overlap = self_right <= other_x
            || other_right <= self.x
            || self_bottom <= other_y
            || other_bottom <= self.y;

        !no_overlap
    }
}

#[derive(Debug, Clone)]
pub struct EdgePath {
    pub source_step: usize,
    pub target_step: usize,
    pub start: [f64; 2],
    pub cp1: [f64; 2],
    pub cp2: [f64; 2],
    pub end: [f64; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EdgeType {
    Sequential,
    Branch { condition_index: usize },
    ErrorRoute,
    RetryRoute,
    JoinRoute,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NodeBadge {
    ActionId(u16),
    RetryMax(u16),
    Timeout(u32),
    SecretSensitive,
    StrictDurable,
    RecentFailures(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowEdge {
    pub from_step: StepIdx,
    pub to_step: StepIdx,
    pub edge_type: EdgeType,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowNode {
    pub step_idx: StepIdx,
    pub kind_name: String,
    pub visual: NodeVisual,
    pub position: Option<(f64, f64)>,
    pub badges: Vec<NodeBadge>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowGraph {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub entry_step: StepIdx,
    pub slot_count: u16,
    pub workflow_name: String,
}
