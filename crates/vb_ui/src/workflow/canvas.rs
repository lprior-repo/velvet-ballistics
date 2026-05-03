//! Workflow canvas -- viewport, selection, focus-jump, and edge path computation.
//!
//! The [`WorkflowCanvas`] holds a flow document and its computed layout, tracks
//! viewport state (pan, zoom), node selection, and provides methods to compute
//! visible node rectangles, center the viewport on a specific node, and build
//! edge paths between connected nodes.

use std::collections::HashMap;

use crate::graph_builder::{FlowDocument, FlowEdgeRecord, FlowNodeRecord};
use crate::layout::{self, LayoutEdge, LayoutNode, LayoutResult};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default zoom level (1.0 = 100%).
const DEFAULT_ZOOM: f64 = 1.0;
/// Minimum zoom level.
const MIN_ZOOM: f64 = 0.1;
/// Maximum zoom level.
const MAX_ZOOM: f64 = 5.0;
/// Bezier control-point offset for edge paths (pixels).
const BEZIER_OFFSET: f64 = 60.0;

// ---------------------------------------------------------------------------
// Viewport rectangle
// ---------------------------------------------------------------------------

/// Axis-aligned rectangle describing the visible viewport region in world
/// coordinates.
#[derive(Debug, Clone, Copy)]
pub struct ViewportRect {
    /// Left edge in world coordinates.
    pub x: f64,
    /// Top edge in world coordinates.
    pub y: f64,
    /// Viewport width in world coordinates.
    pub width: f64,
    /// Viewport height in world coordinates.
    pub height: f64,
}

impl ViewportRect {
    /// Returns `true` if the given rectangle intersects this viewport.
    ///
    /// Two rectangles intersect when they overlap on both axes.
    #[must_use]
    pub fn intersects(&self, other_x: f64, other_y: f64, other_w: f64, other_h: f64) -> bool {
        let self_right = self.x.saturating_add(self.width);
        let self_bottom = self.y.saturating_add(self.height);
        let other_right = other_x.saturating_add(other_w);
        let other_bottom = other_y.saturating_add(other_h);

        // No overlap if one is completely to the left/right/above/below the other.
        let no_overlap = self_right <= other_x
            || other_right <= self.x
            || self_bottom <= other_y
            || other_bottom <= self.y;

        !no_overlap
    }
}

// ---------------------------------------------------------------------------
// Edge path
// ---------------------------------------------------------------------------

/// A cubic Bezier edge path between two node centres.
#[derive(Debug, Clone)]
pub struct EdgePath {
    /// Source step index.
    pub source_step: usize,
    /// Target step index.
    pub target_step: usize,
    /// Start point (centre of source node).
    pub start: [f64; 2],
    /// First control point.
    pub cp1: [f64; 2],
    /// Second control point.
    pub cp2: [f64; 2],
    /// End point (centre of target node).
    pub end: [f64; 2],
}

// ---------------------------------------------------------------------------
// WorkflowCanvas
// ---------------------------------------------------------------------------

/// The workflow authoring canvas.
///
/// Holds a flow document, computed layout positions, viewport state, and node
/// selection. All methods are pure functions that return new values without
/// side effects.
#[derive(Debug, Clone)]
pub struct WorkflowCanvas {
    /// The flow document being displayed.
    document: FlowDocument,
    /// Pre-computed layout positions.
    layout: LayoutResult,
    /// Viewport horizontal pan offset in world coordinates.
    pan_x: f64,
    /// Viewport vertical pan offset in world coordinates.
    pan_y: f64,
    /// Zoom level (1.0 = 100%).
    zoom: f64,
    /// Currently selected node index (step index), if any.
    selected: Option<usize>,
    /// Ordered node IDs from the document (cached for fast lookup).
    node_ids: Vec<String>,
}

impl WorkflowCanvas {
    /// Create a new canvas from a flow document.
    ///
    /// Computes layout positions using the Sugiyama algorithm. The entry node
    /// is determined from `document.graph.entry_node`.
    #[must_use]
    pub fn new(document: FlowDocument) -> Self {
        let entry_id = document
            .graph
            .entry_node
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");

        // Build layout inputs from the document.
        let (layout_nodes, node_ids) = Self::build_layout_nodes(&document);
        let layout_edges = Self::build_layout_edges(&document);

        let layout = layout::compute_layout(&layout_nodes, &layout_edges, entry_id);

        Self {
            document,
            layout,
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: DEFAULT_ZOOM,
            selected: None,
            node_ids,
        }
    }

    /// Returns a reference to the flow document.
    #[must_use]
    pub fn document(&self) -> &FlowDocument {
        &self.document
    }

    /// Returns a reference to the computed layout.
    #[must_use]
    pub fn layout(&self) -> &LayoutResult {
        &self.layout
    }

    /// Returns the current pan offset.
    #[must_use]
    pub fn pan(&self) -> (f64, f64) {
        (self.pan_x, self.pan_y)
    }

    /// Returns the current zoom level.
    #[must_use]
    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    /// Returns the selected node step index, if any.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Set the pan offset.
    pub fn set_pan(&mut self, x: f64, y: f64) {
        self.pan_x = x;
        self.pan_y = y;
    }

    /// Set the zoom level, clamped to `[MIN_ZOOM, MAX_ZOOM]`.
    pub fn set_zoom(&mut self, zoom: f64) {
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// Select a node by step index. Pass `None` to deselect.
    pub fn set_selected(&mut self, step: Option<usize>) {
        self.selected = step;
    }

    /// Compute the visible viewport rectangle in world coordinates.
    ///
    /// The viewport is derived from pan offset, zoom level, and the given
    /// screen dimensions.
    #[must_use]
    pub fn viewport_rect(&self, screen_width: f64, screen_height: f64) -> ViewportRect {
        let inv_zoom = if self.zoom > 0.0 { 1.0 / self.zoom } else { 1.0 };
        ViewportRect {
            x: self.pan_x,
            y: self.pan_y,
            width: screen_width * inv_zoom,
            height: screen_height * inv_zoom,
        }
    }

    /// Compute the visible node rectangles.
    ///
    /// Returns a list of `(step_index, x, y, width, height)` for each node
    /// that intersects the given viewport rectangle.
    #[must_use]
    pub fn visible_nodes(&self, viewport: &ViewportRect) -> Vec<(usize, f64, f64, f64, f64)> {
        let mut result = Vec::new();
        for (idx, node_id) in self.node_ids.iter().enumerate() {
            let pos = match self.layout.positions.get(node_id.as_str()) {
                Some(&p) => p,
                None => continue,
            };
            let node = match self.document.graph.nodes.get(node_id.as_str()) {
                Some(n) => n,
                None => continue,
            };

            let half_w = node.size[0] / 2.0;
            let half_h = node.size[1] / 2.0;

            // Node bounding box (top-left corner).
            let nx = pos[0] - half_w;
            let ny = pos[1] - half_h;

            if viewport.intersects(nx, ny, node.size[0], node.size[1]) {
                result.push((idx, pos[0], pos[1], node.size[0], node.size[1]));
            }
        }
        result
    }

    /// Center the viewport on a specific node by step index.
    ///
    /// Updates `pan_x` and `pan_y` so that the node is centered in a
    /// viewport of the given screen dimensions. Returns `false` if the
    /// step index does not correspond to a valid node.
    pub fn focus_jump(&mut self, step_id: usize, screen_width: f64, screen_height: f64) -> bool {
        let node_id = match self.node_ids.get(step_id) {
            Some(id) => id.as_str(),
            None => return false,
        };

        let pos = match self.layout.positions.get(node_id) {
            Some(&p) => p,
            None => return false,
        };

        let inv_zoom = if self.zoom > 0.0 { 1.0 / self.zoom } else { 1.0 };
        let view_w = screen_width * inv_zoom;
        let view_h = screen_height * inv_zoom;

        // Center the node in the viewport.
        self.pan_x = pos[0] - view_w / 2.0;
        self.pan_y = pos[1] - view_h / 2.0;
        true
    }

    /// Compute cubic Bezier edge paths for all edges in the document.
    ///
    /// Each edge is represented as a horizontal Bezier curve from the
    /// centre-right of the source node to the centre-left of the target
    /// node. The control-point offset is scaled by the horizontal distance
    /// between nodes.
    #[must_use]
    pub fn compute_edge_paths(&self) -> Vec<EdgePath> {
        let mut paths = Vec::new();
        for edge in self.document.graph.edges.values() {
            let (src_step, src_pos, src_size) = match self.resolve_node(&edge.source) {
                Some(v) => v,
                None => continue,
            };
            let (tgt_step, tgt_pos, tgt_size) = match self.resolve_node(&edge.target) {
                Some(v) => v,
                None => continue,
            };

            let start = [
                src_pos[0].saturating_add(src_size[0] / 2.0),
                src_pos[1],
            ];
            let end = [
                tgt_pos[0].saturating_sub(tgt_size[0] / 2.0),
                tgt_pos[1],
            ];

            // Scale the control-point offset by horizontal distance.
            let dx = (end[0] - start[0]).abs();
            let cp_offset = BEZIER_OFFSET.min(dx / 2.0).max(BEZIER_OFFSET / 2.0);

            let cp1 = [start[0].saturating_add(cp_offset), start[1]];
            let cp2 = [end[0].saturating_sub(cp_offset), end[1]];

            paths.push(EdgePath {
                source_step: src_step,
                target_step: tgt_step,
                start,
                cp1,
                cp2,
                end,
            });
        }
        paths
    }

    /// Returns the total number of nodes in the document.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.node_ids.len()
    }

    /// Returns the total number of edges in the document.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.document.graph.edges.len()
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Resolve a node ID string to its step index, position, and size.
    fn resolve_node(&self, node_id: &str) -> Option<(usize, [f64; 2], [f64; 2])> {
        let step_idx = self
            .node_ids
            .iter()
            .position(|id| id.as_str() == node_id)?;

        let pos = self.layout.positions.get(node_id)?;
        let node = self.document.graph.nodes.get(node_id)?;
        Some((step_idx, *pos, node.size))
    }

    /// Build layout node descriptors from the flow document.
    fn build_layout_nodes(document: &FlowDocument) -> (Vec<LayoutNode>, Vec<String>) {
        let mut layout_nodes = Vec::with_capacity(document.graph.nodes.len());
        let mut node_ids = Vec::with_capacity(document.graph.nodes.len());

        for (key, node) in &document.graph.nodes {
            let group = node.parent.as_ref().map(|g| g.as_str().to_string());
            layout_nodes.push(LayoutNode {
                id: key.to_string(),
                width: node.size[0],
                height: node.size[1],
                group,
            });
            node_ids.push(key.to_string());
        }

        (layout_nodes, node_ids)
    }

    /// Build layout edge descriptors from the flow document.
    fn build_layout_edges(document: &FlowDocument) -> Vec<LayoutEdge> {
        let mut layout_edges = Vec::with_capacity(document.graph.edges.len());
        for edge in document.graph.edges.values() {
            layout_edges.push(LayoutEdge {
                source: edge.source.to_string(),
                target: edge.target.to_string(),
            });
        }
        layout_edges
    }

    /// Build a position lookup map from the layout result.
    fn position_map(&self) -> HashMap<usize, [f64; 2]> {
        let mut map = HashMap::new();
        for (idx, node_id) in self.node_ids.iter().enumerate() {
            if let Some(&pos) = self.layout.positions.get(node_id.as_str()) {
                map.insert(idx, pos);
            }
        }
        map
    }

    // Expose for testing: get the position map.
    #[cfg(test)]
    pub fn test_positions(&self) -> HashMap<usize, [f64; 2]> {
        self.position_map()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_builder::{EdgeStyle, FlowEdgeRecord, FlowGraph, FlowNodeRecord, NodeFlags, NodeUiState, build_document};
    use indexmap::IndexMap;
    use smol_str::SmolStr;
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, WorkflowParts};
    use vb_core::ids::WorkflowDigest;

    fn make_nop_node(id: u16, next: Option<u16>) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: next.map(StepIdx::new),
            kind: CompiledNodeKind::Nop,
        }
    }

    fn make_finish_node(id: u16, result_slot: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(result_slot),
            },
        }
    }

    fn make_simple_parts(nodes: Vec<CompiledNode>, entry: u16) -> WorkflowParts {
        let node_count = nodes.len();
        let step_names: Vec<Box<str>> = (0..node_count)
            .map(|i| format!("step-{i}").into_boxed_str())
            .collect();
        WorkflowParts {
            name: String::from("test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            entry: StepIdx::new(entry),
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            step_names: step_names.into_boxed_slice(),
        }
    }

    fn make_empty_document() -> FlowDocument {
        let node = make_finish_node(0, 0);
        let parts = make_simple_parts(vec![node], 0);
        build_document(&parts)
    }

    fn make_chain_document() -> FlowDocument {
        let n0 = make_nop_node(0, Some(1));
        let n1 = make_nop_node(1, Some(2));
        let n2 = make_finish_node(2, 0);
        let parts = make_simple_parts(vec![n0, n1, n2], 0);
        build_document(&parts)
    }

    #[test]
    fn new_canvas_has_default_viewport_state() {
        let doc = make_empty_document();
        let canvas = WorkflowCanvas::new(doc);
        assert_eq!(canvas.pan(), (0.0, 0.0));
        assert!((canvas.zoom() - 1.0).abs() < f64::EPSILON);
        assert!(canvas.selected().is_none());
        assert_eq!(canvas.node_count(), 1);
    }

    #[test]
    fn set_pan_updates_pan() {
        let doc = make_empty_document();
        let mut canvas = WorkflowCanvas::new(doc);
        canvas.set_pan(10.0, 20.0);
        assert_eq!(canvas.pan(), (10.0, 20.0));
    }

    #[test]
    fn set_zoom_clamps_to_range() {
        let doc = make_empty_document();
        let mut canvas = WorkflowCanvas::new(doc);

        canvas.set_zoom(0.01);
        assert!((canvas.zoom() - MIN_ZOOM).abs() < f64::EPSILON);

        canvas.set_zoom(100.0);
        assert!((canvas.zoom() - MAX_ZOOM).abs() < f64::EPSILON);

        canvas.set_zoom(2.0);
        assert!((canvas.zoom() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn set_selected_updates_selection() {
        let doc = make_empty_document();
        let mut canvas = WorkflowCanvas::new(doc);
        assert!(canvas.selected().is_none());
        canvas.set_selected(Some(0));
        assert_eq!(canvas.selected(), Some(0));
        canvas.set_selected(None);
        assert!(canvas.selected().is_none());
    }

    #[test]
    fn viewport_rect_computes_world_bounds() {
        let doc = make_empty_document();
        let mut canvas = WorkflowCanvas::new(doc);
        canvas.set_pan(50.0, 100.0);
        canvas.set_zoom(2.0);

        let vr = canvas.viewport_rect(800.0, 600.0);
        assert!((vr.x - 50.0).abs() < f64::EPSILON);
        assert!((vr.y - 100.0).abs() < f64::EPSILON);
        // At zoom 2.0, screen coords are halved in world space.
        assert!((vr.width - 400.0).abs() < f64::EPSILON);
        assert!((vr.height - 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn viewport_rect_intersects_overlapping() {
        let vr = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        // Partially overlapping.
        assert!(vr.intersects(50.0, 50.0, 100.0, 100.0));
        // Fully inside.
        assert!(vr.intersects(10.0, 10.0, 20.0, 20.0));
        // Same rect.
        assert!(vr.intersects(0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn viewport_rect_no_intersection_when_disjoint() {
        let vr = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        // Completely to the right.
        assert!(!vr.intersects(200.0, 0.0, 100.0, 100.0));
        // Completely below.
        assert!(!vr.intersects(0.0, 200.0, 100.0, 100.0));
        // Completely above.
        assert!(!vr.intersects(0.0, -200.0, 100.0, 100.0));
        // Completely to the left.
        assert!(!vr.intersects(-200.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn visible_nodes_returns_intersecting_nodes() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);

        // Use a large viewport that should contain all nodes.
        let viewport = ViewportRect {
            x: -1000.0,
            y: -1000.0,
            width: 5000.0,
            height: 5000.0,
        };
        let visible = canvas.visible_nodes(&viewport);
        assert_eq!(visible.len(), 3);
    }

    #[test]
    fn visible_nodes_excludes_offscreen_nodes() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);

        // Use a tiny viewport far away from all nodes.
        let viewport = ViewportRect {
            x: -10000.0,
            y: -10000.0,
            width: 1.0,
            height: 1.0,
        };
        let visible = canvas.visible_nodes(&viewport);
        assert!(visible.is_empty());
    }

    #[test]
    fn focus_jump_centers_on_node() {
        let doc = make_chain_document();
        let mut canvas = WorkflowCanvas::new(doc);

        let positions = canvas.test_positions();
        let target_pos = positions.get(&1).copied();
        assert!(target_pos.is_some());
        let target_pos = target_pos.unwrap_or([0.0; 2]);

        let ok = canvas.focus_jump(1, 800.0, 600.0);
        assert!(ok);

        // Pan should center the node.
        let inv_zoom = 1.0 / canvas.zoom();
        let expected_x = target_pos[0] - 800.0 * inv_zoom / 2.0;
        let expected_y = target_pos[1] - 600.0 * inv_zoom / 2.0;
        assert!((canvas.pan().0 - expected_x).abs() < 0.01);
        assert!((canvas.pan().1 - expected_y).abs() < 0.01);
    }

    #[test]
    fn focus_jump_returns_false_for_invalid_step() {
        let doc = make_chain_document();
        let mut canvas = WorkflowCanvas::new(doc);

        let ok = canvas.focus_jump(999, 800.0, 600.0);
        assert!(!ok);
    }

    #[test]
    fn compute_edge_paths_produces_paths_for_chain() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);

        let paths = canvas.compute_edge_paths();
        // Two edges: step-0 -> step-1 (next), step-1 -> step-2 (next).
        assert_eq!(paths.len(), 2);

        // Verify first edge goes from step 0 to step 1.
        let first = &paths[0];
        assert_eq!(first.source_step, 0);
        assert_eq!(first.target_step, 1);

        // Start should be to the right of the source centre.
        assert!(first.start[0] > 0.0);
        // End should be to the left of the target centre.
        // The target node is further right, so end.x > start.x.
        assert!(first.end[0] > first.start[0]);
    }

    #[test]
    fn edge_path_control_points_are_between_start_and_end() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);

        let paths = canvas.compute_edge_paths();
        for path in &paths {
            // cp1.x should be between start.x and end.x.
            assert!(path.cp1[0] >= path.start[0]);
            assert!(path.cp2[0] <= path.end[0]);
            // Control points should not be past the endpoints.
            assert!(path.cp1[0] <= path.end[0]);
            assert!(path.cp2[0] >= path.start[0]);
        }
    }

    #[test]
    fn chain_layout_positions_increase_in_x() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);
        let positions = canvas.test_positions();

        let p0 = positions.get(&0).copied();
        let p1 = positions.get(&1).copied();
        let p2 = positions.get(&2).copied();

        assert!(p0.is_some());
        assert!(p1.is_some());
        assert!(p2.is_some());

        let p0 = p0.unwrap_or([0.0; 2]);
        let p1 = p1.unwrap_or([0.0; 2]);
        let p2 = p2.unwrap_or([0.0; 2]);

        // Nodes should be laid out left to right.
        assert!(p0[0] < p1[0]);
        assert!(p1[0] < p2[0]);
    }

    #[test]
    fn edge_count_matches_document() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);
        assert_eq!(canvas.edge_count(), 2);
    }

    #[test]
    fn node_count_matches_document() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);
        assert_eq!(canvas.node_count(), 3);
    }
}
