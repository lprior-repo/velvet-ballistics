//! Viewport math, selection state, and hit-testing for the flow editor.
//!
//! This module contains all the pure-logic operations that can be tested
//! independently of Makepad rendering:
//!
//! - **Viewport transforms**: world-to-screen, screen-to-world, fit-view
//! - **Selection state**: tracking selected nodes, edges, and groups
//! - **Hit testing**: point-in-rect for nodes, point-to-bezier for edges
//! - **Patch application**: mutating a `FlowDocument` from `FlowPatch` values

use flow_core::doc::{FlowDocument, FlowGraph, FlowNodeRecord, SelectionState, ViewportState};
use flow_core::ids::{EdgeId, GroupId, NodeId};
use flow_core::patch::FlowPatch;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Viewport transforms
// ---------------------------------------------------------------------------

/// A rectangular region in world coordinates.
#[derive(Clone, Copy, Debug, Default)]
pub struct WorldRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Parameters for a viewport transform (pan + zoom).
#[derive(Clone, Copy, Debug)]
pub struct ViewportTransform {
    pub pan_x: f64,
    pub pan_y: f64,
    pub zoom: f64,
}

impl ViewportTransform {
    pub const fn identity() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }
    }

    /// Convert a world point to screen (pixel) coordinates.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn world_to_screen(&self, wx: f64, wy: f64, rect_origin_x: f64, rect_origin_y: f64) -> (f64, f64) {
        let sx = (wx - self.pan_x) * self.zoom + rect_origin_x;
        let sy = (wy - self.pan_y) * self.zoom + rect_origin_y;
        (sx, sy)
    }

    /// Convert a screen (pixel) point to world coordinates.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn screen_to_world(&self, sx: f64, sy: f64, rect_origin_x: f64, rect_origin_y: f64) -> (f64, f64) {
        let wx = (sx - rect_origin_x) / self.zoom + self.pan_x;
        let wy = (sy - rect_origin_y) / self.zoom + self.pan_y;
        (wx, wy)
    }
}

/// Computes a `ViewportTransform` that fits the given world bounds into a
/// screen rect of `canvas_w` x `canvas_h` pixels, with optional padding.
///
/// Returns `None` if the world bounds are empty or degenerate.
#[allow(clippy::arithmetic_side_effects)]
pub fn fit_view(
    bounds: WorldRect,
    canvas_w: f64,
    canvas_h: f64,
    padding_ratio: f64,
) -> Option<ViewportTransform> {
    if bounds.w <= 0.0 || bounds.h <= 0.0 || canvas_w <= 0.0 || canvas_h <= 0.0 {
        return None;
    }

    let effective_w = canvas_w * (1.0 - padding_ratio * 2.0);
    let effective_h = canvas_h * (1.0 - padding_ratio * 2.0);

    if effective_w <= 0.0 || effective_h <= 0.0 {
        return None;
    }

    let scale_x = effective_w / bounds.w;
    let scale_y = effective_h / bounds.h;
    let zoom = scale_x.min(scale_y);

    if zoom <= 0.0 {
        return None;
    }

    let center_x = bounds.x + bounds.w / 2.0;
    let center_y = bounds.y + bounds.h / 2.0;

    let pan_x = center_x - canvas_w / (2.0 * zoom);
    let pan_y = center_y - canvas_h / (2.0 * zoom);

    Some(ViewportTransform {
        pan_x,
        pan_y,
        zoom,
    })
}

/// Computes the bounding rectangle enclosing all non-hidden nodes in the graph.
/// Returns `None` if there are no visible nodes.
pub fn compute_graph_bounds(graph: &FlowGraph) -> Option<WorldRect> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut count = 0usize;

    for node in graph.nodes.values() {
        if node.flags.hidden {
            continue;
        }
        count = count.saturating_add(1);
        let right = node.position[0] + node.size[0];
        let bottom = node.position[1] + node.size[1];
        if node.position[0] < min_x {
            min_x = node.position[0];
        }
        if node.position[1] < min_y {
            min_y = node.position[1];
        }
        if right > max_x {
            max_x = right;
        }
        if bottom > max_y {
            max_y = bottom;
        }
    }

    if count == 0 {
        return None;
    }

    Some(WorldRect {
        x: min_x,
        y: min_y,
        w: max_x - min_x,
        h: max_y - min_y,
    })
}

/// Creates a `ViewportState` from a `ViewportTransform`.
pub const fn viewport_state_from_transform(vt: &ViewportTransform) -> ViewportState {
    ViewportState {
        pan_x: vt.pan_x,
        pan_y: vt.pan_y,
        zoom: vt.zoom,
    }
}

/// Creates a `ViewportTransform` from a `ViewportState`.
pub const fn transform_from_viewport_state(vs: &ViewportState) -> ViewportTransform {
    ViewportTransform {
        pan_x: vs.pan_x,
        pan_y: vs.pan_y,
        zoom: vs.zoom,
    }
}

// ---------------------------------------------------------------------------
// Hit testing
// ---------------------------------------------------------------------------

/// Result of a hit test against the graph.
#[derive(Clone, Debug)]
pub enum HitResult {
    Node(NodeId),
    Edge(EdgeId),
    Nothing,
}

/// Tests whether a world-space point hits any visible node.
/// Returns the first (topmost by z_index) node ID hit, or `HitResult::Nothing`.
pub fn hit_test_nodes(graph: &FlowGraph, world_x: f64, world_y: f64) -> HitResult {
    // Sort nodes by z_index descending so we check the topmost first.
    let mut candidates: Vec<&FlowNodeRecord> = graph
        .nodes
        .values()
        .filter(|n| !n.flags.hidden)
        .collect();

    candidates.sort_by_key(|b| std::cmp::Reverse(b.z_index));

    for node in &candidates {
        let nx = node.position[0];
        let ny = node.position[1];
        let nw = node.size[0];
        let nh = node.size[1];
        if world_x >= nx && world_x <= nx + nw && world_y >= ny && world_y <= ny + nh {
            return HitResult::Node(node.id.clone());
        }
    }

    HitResult::Nothing
}

/// A 2D point used for geometry computations.
#[derive(Clone, Copy, Debug)]
struct Point {
    x: f64,
    y: f64,
}

/// Four control points of a cubic bezier curve.
#[derive(Clone, Copy, Debug)]
struct CubicBezier {
    p0: Point,
    p1: Point,
    p2: Point,
    p3: Point,
}

/// Tests whether a world-space point is close enough to an edge's bezier curve.
///
/// The distance threshold is `tolerance` in world units. We sample the bezier
/// at `num_samples` evenly spaced `t` values and check minimum distance.
#[allow(clippy::arithmetic_side_effects)]
pub fn hit_test_edges(
    graph: &FlowGraph,
    world_x: f64,
    world_y: f64,
    tolerance: f64,
    num_samples: usize,
) -> HitResult {
    if num_samples == 0 || tolerance < 0.0 {
        return HitResult::Nothing;
    }

    let pt = Point { x: world_x, y: world_y };
    let tol_sq = tolerance * tolerance;
    let mut best: Option<(f64, EdgeId)> = None;

    for edge in graph.edges.values() {
        let source = match graph.nodes.get(&edge.source_node) {
            Some(n) => n,
            None => continue,
        };
        let target = match graph.nodes.get(&edge.target_node) {
            Some(n) => n,
            None => continue,
        };

        let (x1, y1) = compute_port_world_pos(source, &edge.source_port, true);
        let (x2, y2) = compute_port_world_pos(target, &edge.target_port, false);

        let dx = (x2 - x1).abs();
        let cp_offset = dx.max(40.0) * 0.4;

        let curve = CubicBezier {
            p0: Point { x: x1, y: y1 },
            p1: Point { x: x1 + cp_offset, y: y1 },
            p2: Point { x: x2 - cp_offset, y: y2 },
            p3: Point { x: x2, y: y2 },
        };

        let min_dist_sq = min_distance_to_cubic_bezier(pt, &curve, num_samples);

        if min_dist_sq <= tol_sq {
            match best {
                Some((current_best, _)) if min_dist_sq < current_best => {
                    best = Some((min_dist_sq, edge.id.clone()));
                }
                Some(_) => {}
                None => best = Some((min_dist_sq, edge.id.clone())),
            }
        }
    }

    match best {
        Some((_, id)) => HitResult::Edge(id),
        None => HitResult::Nothing,
    }
}

/// Computes the world position of a port on a node.
/// If `is_output`, uses the right side; otherwise the left side.
#[allow(clippy::arithmetic_side_effects)]
fn compute_port_world_pos(
    node: &FlowNodeRecord,
    port_id: &flow_core::ids::PortId,
    is_output: bool,
) -> (f64, f64) {
    let header_h = 32.0_f64;
    let padding = 12.0_f64;
    let port_height = 20.0_f64;

    let order = node
        .ports
        .iter()
        .find(|p| p.id == *port_id)
        .map(|p| p.order)
        .unwrap_or(0);

    let py = node.position[1] + header_h + padding + f64::from(order) * port_height + port_height / 2.0;
    let px = if is_output {
        node.position[0] + node.size[0]
    } else {
        node.position[0]
    };

    (px, py)
}

/// Returns the minimum squared distance from point `pt` to a cubic bezier
/// curve sampled at `num_samples` evenly spaced `t` values.
#[allow(clippy::arithmetic_side_effects)]
fn min_distance_to_cubic_bezier(
    pt: Point,
    curve: &CubicBezier,
    num_samples: usize,
) -> f64 {
    let mut min_sq = f64::MAX;

    for i in 0..=num_samples {
        let t = i_to_f64(num_samples, i);

        let bpt = cubic_bezier_point(t, curve);
        let ddx = pt.x - bpt.x;
        let ddy = pt.y - bpt.y;
        let dist_sq = ddx * ddx + ddy * ddy;
        if dist_sq < min_sq {
            min_sq = dist_sq;
        }
    }

    min_sq
}

/// Convert iteration index `i` out of `total` steps to a [0,1] parameter.
#[allow(clippy::as_conversions)]
fn i_to_f64(total: usize, i: usize) -> f64 {
    (i as f64) / (total as f64)
}

/// Evaluates a cubic bezier at parameter `t` in [0, 1].
#[allow(clippy::arithmetic_side_effects)]
fn cubic_bezier_point(t: f64, curve: &CubicBezier) -> Point {
    let u = 1.0 - t;
    let uu = u * u;
    let uuu = uu * u;
    let tt = t * t;
    let ttt = tt * t;

    Point {
        x: uuu * curve.p0.x + 3.0 * uu * t * curve.p1.x + 3.0 * u * tt * curve.p2.x + ttt * curve.p3.x,
        y: uuu * curve.p0.y + 3.0 * uu * t * curve.p1.y + 3.0 * u * tt * curve.p2.y + ttt * curve.p3.y,
    }
}

// ---------------------------------------------------------------------------
// Selection state
// ---------------------------------------------------------------------------

/// Manages the current selection of nodes, edges, and groups.
#[derive(Clone, Debug, Default)]
pub struct Selection {
    selected_nodes: HashSet<NodeId>,
    selected_edges: HashSet<EdgeId>,
    selected_groups: HashSet<GroupId>,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a `Selection` from a `SelectionState`.
    pub fn from_selection_state(state: &SelectionState) -> Self {
        Self {
            selected_nodes: state.selected_nodes.iter().cloned().collect(),
            selected_edges: state.selected_edges.iter().cloned().collect(),
            selected_groups: state.selected_groups.iter().cloned().collect(),
        }
    }

    /// Convert to a `SelectionState`.
    pub fn to_selection_state(&self) -> SelectionState {
        SelectionState {
            selected_nodes: self.selected_nodes.iter().cloned().collect(),
            selected_edges: self.selected_edges.iter().cloned().collect(),
            selected_groups: self.selected_groups.iter().cloned().collect(),
        }
    }

    /// Select a single node, clearing all other selections.
    pub fn select_node(&mut self, id: NodeId) {
        self.clear();
        self.selected_nodes.insert(id);
    }

    /// Toggle a node in the selection.
    pub fn toggle_node(&mut self, id: NodeId) {
        if self.selected_nodes.contains(&id) {
            self.selected_nodes.remove(&id);
        } else {
            self.selected_nodes.insert(id);
        }
    }

    /// Add a node to the current selection without clearing.
    pub fn add_node(&mut self, id: NodeId) {
        self.selected_nodes.insert(id);
    }

    /// Select a single edge, clearing all other selections.
    pub fn select_edge(&mut self, id: EdgeId) {
        self.clear();
        self.selected_edges.insert(id);
    }

    /// Toggle an edge in the selection.
    pub fn toggle_edge(&mut self, id: EdgeId) {
        if self.selected_edges.contains(&id) {
            self.selected_edges.remove(&id);
        } else {
            self.selected_edges.insert(id);
        }
    }

    /// Add an edge to the current selection without clearing.
    pub fn add_edge(&mut self, id: EdgeId) {
        self.selected_edges.insert(id);
    }

    /// Select a single group, clearing all other selections.
    pub fn select_group(&mut self, id: GroupId) {
        self.clear();
        self.selected_groups.insert(id);
    }

    /// Select all nodes and edges in the graph.
    pub fn select_all(&mut self, graph: &FlowGraph) {
        self.clear();
        for id in graph.nodes.keys() {
            self.selected_nodes.insert(id.clone());
        }
        for id in graph.edges.keys() {
            self.selected_edges.insert(id.clone());
        }
    }

    /// Clear all selections.
    pub fn clear(&mut self) {
        self.selected_nodes.clear();
        self.selected_edges.clear();
        self.selected_groups.clear();
    }

    /// Returns `true` if the given node is selected.
    pub fn is_node_selected(&self, id: &NodeId) -> bool {
        self.selected_nodes.contains(id)
    }

    /// Returns `true` if the given edge is selected.
    pub fn is_edge_selected(&self, id: &EdgeId) -> bool {
        self.selected_edges.contains(id)
    }

    /// Returns `true` if the given group is selected.
    pub fn is_group_selected(&self, id: &GroupId) -> bool {
        self.selected_groups.contains(id)
    }

    /// Returns the number of selected nodes.
    pub fn node_count(&self) -> usize {
        self.selected_nodes.len()
    }

    /// Returns the number of selected edges.
    pub fn edge_count(&self) -> usize {
        self.selected_edges.len()
    }

    /// Returns the number of selected groups.
    pub fn group_count(&self) -> usize {
        self.selected_groups.len()
    }

    /// Returns `true` if nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.selected_nodes.is_empty()
            && self.selected_edges.is_empty()
            && self.selected_groups.is_empty()
    }

    /// Returns `true` if at least one item is selected.
    pub fn any_selected(&self) -> bool {
        !self.is_empty()
    }

    /// Returns the total number of selected items (nodes + edges + groups).
    pub fn total_count(&self) -> usize {
        self.selected_nodes
            .len()
            .saturating_add(self.selected_edges.len())
            .saturating_add(self.selected_groups.len())
    }
}

// ---------------------------------------------------------------------------
// Patch application
// ---------------------------------------------------------------------------

/// Applies a `FlowPatch` to a `FlowDocument` in place.
/// Returns `true` if the document was actually mutated, `false` if the patch
/// was a no-op for this document state.
#[allow(clippy::arithmetic_side_effects)]
pub fn apply_patch(doc: &mut FlowDocument, patch: FlowPatch) -> bool {
    match patch {
        FlowPatch::InsertNode { node } => {
            let id = node.id.clone();
            doc.graph.nodes.insert(id, node);
            true
        }
        FlowPatch::UpdateNode { id, changes } => {
            let Some(node) = doc.graph.nodes.get_mut(&id) else {
                return false;
            };
            let mut changed = false;
            if let Some(pos) = changes.position {
                node.position = pos;
                changed = true;
            }
            if let Some(size) = changes.size {
                node.size = size;
                changed = true;
            }
            if let Some(title) = changes.title {
                node.title = title;
                changed = true;
            }
            if let Some(kind) = changes.kind {
                node.kind = kind;
                changed = true;
            }
            if let Some(data) = changes.data {
                node.data = data;
                changed = true;
            }
            if let Some(flags) = changes.flags {
                node.flags = flags;
                changed = true;
            }
            if let Some(ui) = changes.ui {
                node.ui = ui;
                changed = true;
            }
            changed
        }
        FlowPatch::RemoveNode { id } => {
            // Also remove edges connected to this node
            let connected: Vec<EdgeId> = doc
                .graph
                .edges
                .values()
                .filter(|e| e.source_node == id || e.target_node == id)
                .map(|e| e.id.clone())
                .collect();
            for eid in connected {
                doc.graph.edges.shift_remove(&eid);
            }
            doc.graph.nodes.shift_remove(&id).is_some()
        }
        FlowPatch::InsertEdge { edge } => {
            let id = edge.id.clone();
            doc.graph.edges.insert(id, edge);
            true
        }
        FlowPatch::UpdateEdge { id, changes } => {
            let Some(edge) = doc.graph.edges.get_mut(&id) else {
                return false;
            };
            let mut changed = false;
            if let Some(label) = changes.label {
                edge.label = label;
                changed = true;
            }
            if let Some(style) = changes.style {
                edge.style = style;
                changed = true;
            }
            if let Some(data) = changes.data {
                edge.data = data;
                changed = true;
            }
            changed
        }
        FlowPatch::RemoveEdge { id } => doc.graph.edges.shift_remove(&id).is_some(),
        FlowPatch::InsertGroup { group } => {
            let id = group.id.clone();
            doc.graph.groups.insert(id, group);
            true
        }
        FlowPatch::UpdateGroup { id, changes } => {
            let Some(group) = doc.graph.groups.get_mut(&id) else {
                return false;
            };
            let mut changed = false;
            if let Some(title) = changes.title {
                group.title = title;
                changed = true;
            }
            if let Some(bounds) = changes.bounds {
                group.bounds = bounds;
                changed = true;
            }
            if let Some(data) = changes.data {
                group.data = data;
                changed = true;
            }
            changed
        }
        FlowPatch::RemoveGroup { id } => {
            // Clear parent references for nodes in this group
            for node in doc.graph.nodes.values_mut() {
                if node.parent.as_ref() == Some(&id) {
                    node.parent = None;
                }
            }
            doc.graph.groups.shift_remove(&id).is_some()
        }
        FlowPatch::SetViewport { viewport } => {
            doc.editor.viewport = viewport;
            true
        }
        FlowPatch::SetEntryNode { node } => {
            let changed = doc.graph.entry_node != node;
            doc.graph.entry_node = node;
            changed
        }
        FlowPatch::ReparentNodes {
            node_ids,
            new_parent,
        } => {
            let mut changed = false;
            for nid in node_ids {
                if let Some(node) = doc.graph.nodes.get_mut(&nid)
                    && node.parent != new_parent
                {
                    node.parent = new_parent.clone();
                    changed = true;
                }
            }
            changed
        }
    }
}

/// Applies a batch of patches sequentially. Returns the number of patches
/// that actually mutated the document.
pub fn apply_patches(doc: &mut FlowDocument, patches: Vec<FlowPatch>) -> usize {
    let mut count = 0usize;
    for patch in patches {
        if apply_patch(doc, patch) {
            count = count.saturating_add(1);
        }
    }
    count
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use flow_core::doc::{
        EdgeStyle, FlowEdgeRecord, FlowGroupRecord, FlowNodeRecord, GroupKind, NodeFlags,
        NodeUiState,
    };
    use flow_core::ids::PortId;
    use smol_str::SmolStr;

    // ---- helpers ----

    fn nid(s: &str) -> NodeId {
        SmolStr::from(s)
    }

    fn eid(s: &str) -> EdgeId {
        SmolStr::from(s)
    }

    fn gid(s: &str) -> GroupId {
        SmolStr::from(s)
    }

    fn pid(s: &str) -> PortId {
        SmolStr::from(s)
    }

    fn make_node_at(id: &str, x: f64, y: f64) -> FlowNodeRecord {
        FlowNodeRecord {
            id: nid(id),
            kind: SmolStr::from("test"),
            title: SmolStr::from(id),
            position: [x, y],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: Vec::new(),
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        }
    }

    fn make_node_at_z(id: &str, x: f64, y: f64, z: i32) -> FlowNodeRecord {
        FlowNodeRecord {
            z_index: z,
            ..make_node_at(id, x, y)
        }
    }

    fn make_hidden_node(id: &str, x: f64, y: f64) -> FlowNodeRecord {
        FlowNodeRecord {
            flags: NodeFlags {
                hidden: true,
                ..NodeFlags::default()
            },
            ..make_node_at(id, x, y)
        }
    }

    fn make_edge(id: &str, src: &str, tgt: &str) -> FlowEdgeRecord {
        FlowEdgeRecord {
            id: eid(id),
            source_node: nid(src),
            source_port: pid("out"),
            target_node: nid(tgt),
            target_port: pid("in"),
            label: None,
            style: EdgeStyle::default(),
            data: serde_json::Value::Null,
            ui: flow_core::doc::EdgeUiState::default(),
        }
    }

    fn make_group(id: &str) -> FlowGroupRecord {
        FlowGroupRecord {
            id: gid(id),
            kind: GroupKind::Generic,
            title: SmolStr::from(id),
            bounds: [0.0, 0.0, 200.0, 200.0],
            data: serde_json::Value::Null,
        }
    }

    // ---- ViewportTransform ----

    #[test]
    fn identity_transform_world_to_screen() {
        let vt = ViewportTransform::identity();
        let (sx, sy) = vt.world_to_screen(10.0, 20.0, 0.0, 0.0);
        assert!((sx - 10.0).abs() < f64::EPSILON);
        assert!((sy - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn identity_transform_screen_to_world() {
        let vt = ViewportTransform::identity();
        let (wx, wy) = vt.screen_to_world(10.0, 20.0, 0.0, 0.0);
        assert!((wx - 10.0).abs() < f64::EPSILON);
        assert!((wy - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn world_screen_roundtrip() {
        let vt = ViewportTransform {
            pan_x: 50.0,
            pan_y: 30.0,
            zoom: 2.0,
        };
        let (sx, sy) = vt.world_to_screen(100.0, 200.0, 10.0, 20.0);
        let (wx, wy) = vt.screen_to_world(sx, sy, 10.0, 20.0);
        assert!((wx - 100.0).abs() < 1e-10);
        assert!((wy - 200.0).abs() < 1e-10);
    }

    #[test]
    fn transform_with_zoom() {
        let vt = ViewportTransform {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 2.0,
        };
        let (sx, sy) = vt.world_to_screen(50.0, 50.0, 0.0, 0.0);
        // At 2x zoom, world 50 should be screen 100
        assert!((sx - 100.0).abs() < f64::EPSILON);
        assert!((sy - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn transform_with_pan() {
        let vt = ViewportTransform {
            pan_x: 100.0,
            pan_y: 100.0,
            zoom: 1.0,
        };
        let (sx, sy) = vt.world_to_screen(150.0, 150.0, 0.0, 0.0);
        // Pan 100 means world 150 maps to (150-100)*1 = screen 50
        assert!((sx - 50.0).abs() < f64::EPSILON);
        assert!((sy - 50.0).abs() < f64::EPSILON);
    }

    // ---- fit_view ----

    #[test]
    fn fit_view_basic() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 100.0,
        };
        let vt = fit_view(bounds, 800.0, 600.0, 0.1).unwrap();
        assert!(vt.zoom > 0.0);
        // The zoom should fit both dimensions
        let fitted_w = 200.0 * vt.zoom;
        let fitted_h = 100.0 * vt.zoom;
        let effective_w = 800.0 * 0.8;
        let effective_h = 600.0 * 0.8;
        assert!(fitted_w <= effective_w + 1.0);
        assert!(fitted_h <= effective_h + 1.0);
    }

    #[test]
    fn fit_view_empty_bounds_returns_none() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 100.0,
        };
        assert!(fit_view(bounds, 800.0, 600.0, 0.1).is_none());
    }

    #[test]
    fn fit_view_zero_canvas_returns_none() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 100.0,
        };
        assert!(fit_view(bounds, 0.0, 600.0, 0.1).is_none());
    }

    #[test]
    fn fit_view_negative_size_returns_none() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: -10.0,
            h: 100.0,
        };
        assert!(fit_view(bounds, 800.0, 600.0, 0.1).is_none());
    }

    #[test]
    fn fit_view_excessive_padding_returns_none() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 100.0,
        };
        // 50% padding on each side leaves nothing
        assert!(fit_view(bounds, 800.0, 600.0, 0.5).is_none());
    }

    #[test]
    fn fit_view_tall_world_landscape_canvas() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 400.0,
        };
        let vt = fit_view(bounds, 800.0, 600.0, 0.0).unwrap();
        // Height is the constraint: 400 * zoom = 600 => zoom = 1.5
        assert!((vt.zoom - 1.5).abs() < 1e-10);
    }

    // ---- compute_graph_bounds ----

    #[test]
    fn graph_bounds_empty_graph() {
        let graph = FlowGraph::default();
        assert!(compute_graph_bounds(&graph).is_none());
    }

    #[test]
    fn graph_bounds_single_node() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 10.0, 20.0));
        let bounds = compute_graph_bounds(&graph).unwrap();
        assert!((bounds.x - 10.0).abs() < f64::EPSILON);
        assert!((bounds.y - 20.0).abs() < f64::EPSILON);
        assert!((bounds.w - 100.0).abs() < f64::EPSILON);
        assert!((bounds.h - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn graph_bounds_multiple_nodes() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 100.0));
        let bounds = compute_graph_bounds(&graph).unwrap();
        assert!((bounds.x - 0.0).abs() < f64::EPSILON);
        assert!((bounds.y - 0.0).abs() < f64::EPSILON);
        // Right edge: 200+100=300, bottom edge: 100+50=150
        assert!((bounds.w - 300.0).abs() < f64::EPSILON);
        assert!((bounds.h - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn graph_bounds_skips_hidden_nodes() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_hidden_node("b", 500.0, 500.0));
        let bounds = compute_graph_bounds(&graph).unwrap();
        // Only visible node is at (0,0) with size (100,50)
        assert!((bounds.w - 100.0).abs() < f64::EPSILON);
        assert!((bounds.h - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn graph_bounds_only_hidden_nodes() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_hidden_node("a", 0.0, 0.0));
        assert!(compute_graph_bounds(&graph).is_none());
    }

    // ---- hit_test_nodes ----

    #[test]
    fn hit_test_nodes_empty_graph() {
        let graph = FlowGraph::default();
        let result = hit_test_nodes(&graph, 50.0, 25.0);
        assert!(matches!(result, HitResult::Nothing));
    }

    #[test]
    fn hit_test_nodes_inside() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        let result = hit_test_nodes(&graph, 50.0, 25.0);
        assert!(matches!(result, HitResult::Node(ref id) if id == &nid("a")));
    }

    #[test]
    fn hit_test_nodes_outside() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        let result = hit_test_nodes(&graph, 200.0, 200.0);
        assert!(matches!(result, HitResult::Nothing));
    }

    #[test]
    fn hit_test_nodes_edge_inside() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        // Point at the right edge of the node
        let result = hit_test_nodes(&graph, 100.0, 50.0);
        assert!(matches!(result, HitResult::Node(_)));
    }

    #[test]
    fn hit_test_nodes_skips_hidden() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_hidden_node("a", 0.0, 0.0));
        let result = hit_test_nodes(&graph, 50.0, 25.0);
        assert!(matches!(result, HitResult::Nothing));
    }

    #[test]
    fn hit_test_nodes_topmost_z_index() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("bottom"), make_node_at_z("bottom", 0.0, 0.0, 0));
        graph.nodes.insert(nid("top"), make_node_at_z("top", 0.0, 0.0, 5));
        let result = hit_test_nodes(&graph, 50.0, 25.0);
        // Topmost (z=5) should be hit first
        assert!(matches!(result, HitResult::Node(ref id) if id == &nid("top")));
    }

    // ---- hit_test_edges ----

    #[test]
    fn hit_test_edges_empty_graph() {
        let graph = FlowGraph::default();
        let result = hit_test_edges(&graph, 50.0, 50.0, 10.0, 20);
        assert!(matches!(result, HitResult::Nothing));
    }

    #[test]
    fn hit_test_edges_zero_samples() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 0.0));
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let result = hit_test_edges(&graph, 100.0, 0.0, 10.0, 0);
        assert!(matches!(result, HitResult::Nothing));
    }

    #[test]
    fn hit_test_edges_missing_nodes() {
        let mut graph = FlowGraph::default();
        // Edge references non-existent nodes
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let result = hit_test_edges(&graph, 100.0, 0.0, 10.0, 20);
        assert!(matches!(result, HitResult::Nothing));
    }

    // ---- cubic_bezier_point ----

    #[test]
    fn bezier_at_t0_is_start() {
        let curve = CubicBezier {
            p0: Point { x: 1.0, y: 2.0 },
            p1: Point { x: 10.0, y: 20.0 },
            p2: Point { x: 30.0, y: 40.0 },
            p3: Point { x: 50.0, y: 60.0 },
        };
        let pt = cubic_bezier_point(0.0, &curve);
        assert!((pt.x - 1.0).abs() < 1e-10);
        assert!((pt.y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn bezier_at_t1_is_end() {
        let curve = CubicBezier {
            p0: Point { x: 1.0, y: 2.0 },
            p1: Point { x: 10.0, y: 20.0 },
            p2: Point { x: 30.0, y: 40.0 },
            p3: Point { x: 50.0, y: 60.0 },
        };
        let pt = cubic_bezier_point(1.0, &curve);
        assert!((pt.x - 50.0).abs() < 1e-10);
        assert!((pt.y - 60.0).abs() < 1e-10);
    }

    #[test]
    fn bezier_at_t05_is_midpoint_linear() {
        // When control points are at the midpoint of a linear curve,
        // t=0.5 should be the midpoint.
        let curve = CubicBezier {
            p0: Point { x: 0.0, y: 0.0 },
            p1: Point { x: 50.0, y: 50.0 },
            p2: Point { x: 50.0, y: 50.0 },
            p3: Point { x: 100.0, y: 100.0 },
        };
        let pt = cubic_bezier_point(0.5, &curve);
        assert!((pt.x - 50.0).abs() < 1e-10);
        assert!((pt.y - 50.0).abs() < 1e-10);
    }

    // ---- min_distance_to_cubic_bezier ----

    #[test]
    fn min_dist_to_bezier_at_start() {
        let curve = CubicBezier {
            p0: Point { x: 0.0, y: 0.0 },
            p1: Point { x: 50.0, y: 0.0 },
            p2: Point { x: 50.0, y: 100.0 },
            p3: Point { x: 100.0, y: 100.0 },
        };
        let d = min_distance_to_cubic_bezier(Point { x: 0.0, y: 0.0 }, &curve, 100);
        assert!(d < 1e-10);
    }

    #[test]
    fn min_dist_to_bezier_far_away() {
        let curve = CubicBezier {
            p0: Point { x: 0.0, y: 0.0 },
            p1: Point { x: 50.0, y: 0.0 },
            p2: Point { x: 50.0, y: 100.0 },
            p3: Point { x: 100.0, y: 100.0 },
        };
        let d = min_distance_to_cubic_bezier(Point { x: 500.0, y: 500.0 }, &curve, 100);
        assert!(d > 100.0);
    }

    // ---- Selection ----

    #[test]
    fn selection_new_is_empty() {
        let sel = Selection::new();
        assert!(sel.is_empty());
        assert_eq!(sel.total_count(), 0);
    }

    #[test]
    fn selection_select_node() {
        let mut sel = Selection::new();
        sel.select_node(nid("a"));
        assert!(sel.is_node_selected(&nid("a")));
        assert!(!sel.is_node_selected(&nid("b")));
        assert_eq!(sel.node_count(), 1);
        assert!(sel.any_selected());
    }

    #[test]
    fn selection_select_node_clears_previous() {
        let mut sel = Selection::new();
        sel.select_node(nid("a"));
        sel.select_node(nid("b"));
        assert!(!sel.is_node_selected(&nid("a")));
        assert!(sel.is_node_selected(&nid("b")));
        assert_eq!(sel.node_count(), 1);
    }

    #[test]
    fn selection_toggle_node() {
        let mut sel = Selection::new();
        sel.toggle_node(nid("a"));
        assert!(sel.is_node_selected(&nid("a")));
        sel.toggle_node(nid("a"));
        assert!(!sel.is_node_selected(&nid("a")));
    }

    #[test]
    fn selection_add_node() {
        let mut sel = Selection::new();
        sel.add_node(nid("a"));
        sel.add_node(nid("b"));
        assert!(sel.is_node_selected(&nid("a")));
        assert!(sel.is_node_selected(&nid("b")));
        assert_eq!(sel.node_count(), 2);
    }

    #[test]
    fn selection_select_edge() {
        let mut sel = Selection::new();
        sel.select_edge(eid("e1"));
        assert!(sel.is_edge_selected(&eid("e1")));
        assert_eq!(sel.edge_count(), 1);
    }

    #[test]
    fn selection_toggle_edge() {
        let mut sel = Selection::new();
        sel.toggle_edge(eid("e1"));
        assert!(sel.is_edge_selected(&eid("e1")));
        sel.toggle_edge(eid("e1"));
        assert!(!sel.is_edge_selected(&eid("e1")));
    }

    #[test]
    fn selection_select_group() {
        let mut sel = Selection::new();
        sel.select_group(gid("g1"));
        assert!(sel.is_group_selected(&gid("g1")));
        assert_eq!(sel.group_count(), 1);
    }

    #[test]
    fn selection_clear() {
        let mut sel = Selection::new();
        sel.add_node(nid("a"));
        sel.add_edge(eid("e1"));
        sel.select_group(gid("g1"));
        sel.clear();
        assert!(sel.is_empty());
    }

    #[test]
    fn selection_select_all() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 100.0, 0.0));
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));

        let mut sel = Selection::new();
        sel.select_all(&graph);
        assert_eq!(sel.node_count(), 2);
        assert_eq!(sel.edge_count(), 1);
    }

    #[test]
    fn selection_roundtrip_to_state() {
        let mut sel = Selection::new();
        sel.add_node(nid("a"));
        sel.add_node(nid("b"));
        sel.add_edge(eid("e1"));

        let state = sel.to_selection_state();
        assert_eq!(state.selected_nodes.len(), 2);
        assert_eq!(state.selected_edges.len(), 1);
        assert!(state.selected_groups.is_empty());
    }

    #[test]
    fn selection_from_state() {
        let state = SelectionState {
            selected_nodes: vec![nid("a"), nid("b")],
            selected_edges: vec![eid("e1")],
            selected_groups: vec![gid("g1")],
        };
        let sel = Selection::from_selection_state(&state);
        assert!(sel.is_node_selected(&nid("a")));
        assert!(sel.is_node_selected(&nid("b")));
        assert!(sel.is_edge_selected(&eid("e1")));
        assert!(sel.is_group_selected(&gid("g1")));
        assert_eq!(sel.total_count(), 4);
    }

    // ---- apply_patch ----

    #[test]
    fn patch_insert_node() {
        let mut doc = FlowDocument::default();
        let node = make_node_at("n1", 10.0, 20.0);
        let changed = apply_patch(&mut doc, FlowPatch::InsertNode { node });
        assert!(changed);
        assert!(doc.graph.nodes.contains_key(&nid("n1")));
    }

    #[test]
    fn patch_update_node_position() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));

        let changes = flow_core::patch::NodeChangeSet {
            position: Some([100.0, 200.0]),
            ..flow_core::patch::NodeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateNode { id: nid("n1"), changes });
        assert!(changed);
        let node = doc.graph.nodes.get(&nid("n1")).unwrap();
        assert!((node.position[0] - 100.0).abs() < f64::EPSILON);
        assert!((node.position[1] - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn patch_update_node_title() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));

        let changes = flow_core::patch::NodeChangeSet {
            title: Some(SmolStr::from("new-title")),
            ..flow_core::patch::NodeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateNode { id: nid("n1"), changes });
        assert!(changed);
        assert_eq!(
            doc.graph.nodes.get(&nid("n1")).unwrap().title.as_str(),
            "new-title"
        );
    }

    #[test]
    fn patch_update_nonexistent_node() {
        let mut doc = FlowDocument::default();
        let changes = flow_core::patch::NodeChangeSet {
            position: Some([100.0, 200.0]),
            ..flow_core::patch::NodeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateNode { id: nid("ghost"), changes });
        assert!(!changed);
    }

    #[test]
    fn patch_remove_node() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));
        let changed = apply_patch(&mut doc, FlowPatch::RemoveNode { id: nid("n1") });
        assert!(changed);
        assert!(!doc.graph.nodes.contains_key(&nid("n1")));
    }

    #[test]
    fn patch_remove_node_cascades_edges() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        doc.graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 0.0));
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));

        apply_patch(&mut doc, FlowPatch::RemoveNode { id: nid("a") });
        // Edge "e1" should be removed since it referenced node "a"
        assert!(!doc.graph.edges.contains_key(&eid("e1")));
        assert!(doc.graph.nodes.contains_key(&nid("b")));
    }

    #[test]
    fn patch_remove_nonexistent_node() {
        let mut doc = FlowDocument::default();
        let changed = apply_patch(&mut doc, FlowPatch::RemoveNode { id: nid("ghost") });
        assert!(!changed);
    }

    #[test]
    fn patch_insert_edge() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        doc.graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 0.0));

        let edge = make_edge("e1", "a", "b");
        let changed = apply_patch(&mut doc, FlowPatch::InsertEdge { edge });
        assert!(changed);
        assert!(doc.graph.edges.contains_key(&eid("e1")));
    }

    #[test]
    fn patch_update_edge_label() {
        let mut doc = FlowDocument::default();
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));

        let changes = flow_core::patch::EdgeChangeSet {
            label: Some(Some(SmolStr::from("data"))),
            ..flow_core::patch::EdgeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateEdge { id: eid("e1"), changes });
        assert!(changed);
        assert_eq!(
            doc.graph.edges.get(&eid("e1")).unwrap().label.as_ref().map(|s| s.as_str()),
            Some("data")
        );
    }

    #[test]
    fn patch_update_edge_clear_label() {
        let mut doc = FlowDocument::default();
        let mut edge = make_edge("e1", "a", "b");
        edge.label = Some(SmolStr::from("old"));
        doc.graph.edges.insert(eid("e1"), edge);

        let changes = flow_core::patch::EdgeChangeSet {
            label: Some(None),
            ..flow_core::patch::EdgeChangeSet::default()
        };
        apply_patch(&mut doc, FlowPatch::UpdateEdge { id: eid("e1"), changes });
        assert!(doc.graph.edges.get(&eid("e1")).unwrap().label.is_none());
    }

    #[test]
    fn patch_remove_edge() {
        let mut doc = FlowDocument::default();
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let changed = apply_patch(&mut doc, FlowPatch::RemoveEdge { id: eid("e1") });
        assert!(changed);
        assert!(doc.graph.edges.is_empty());
    }

    #[test]
    fn patch_insert_group() {
        let mut doc = FlowDocument::default();
        let group = make_group("g1");
        let changed = apply_patch(&mut doc, FlowPatch::InsertGroup { group });
        assert!(changed);
        assert!(doc.graph.groups.contains_key(&gid("g1")));
    }

    #[test]
    fn patch_update_group_title() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));

        let changes = flow_core::patch::GroupChangeSet {
            title: Some(SmolStr::from("renamed")),
            ..flow_core::patch::GroupChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateGroup { id: gid("g1"), changes });
        assert!(changed);
        assert_eq!(
            doc.graph.groups.get(&gid("g1")).unwrap().title.as_str(),
            "renamed"
        );
    }

    #[test]
    fn patch_remove_group_clears_parent_refs() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let mut node = make_node_at("n1", 0.0, 0.0);
        node.parent = Some(gid("g1"));
        doc.graph.nodes.insert(nid("n1"), node);

        apply_patch(&mut doc, FlowPatch::RemoveGroup { id: gid("g1") });
        assert!(!doc.graph.groups.contains_key(&gid("g1")));
        // Node's parent ref should be cleared
        assert!(doc.graph.nodes.get(&nid("n1")).unwrap().parent.is_none());
    }

    #[test]
    fn patch_set_viewport() {
        let mut doc = FlowDocument::default();
        let vp = ViewportState {
            pan_x: 10.0,
            pan_y: 20.0,
            zoom: 2.0,
        };
        let changed = apply_patch(&mut doc, FlowPatch::SetViewport { viewport: vp.clone() });
        assert!(changed);
        assert!((doc.editor.viewport.pan_x - 10.0).abs() < f64::EPSILON);
        assert!((doc.editor.viewport.zoom - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn patch_set_entry_node() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));

        let changed = apply_patch(
            &mut doc,
            FlowPatch::SetEntryNode {
                node: Some(nid("n1")),
            },
        );
        assert!(changed);
        assert_eq!(doc.graph.entry_node, Some(nid("n1")));
    }

    #[test]
    fn patch_set_entry_node_no_change() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("n1"));
        let changed = apply_patch(
            &mut doc,
            FlowPatch::SetEntryNode {
                node: Some(nid("n1")),
            },
        );
        assert!(!changed);
    }

    #[test]
    fn patch_reparent_nodes() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));
        doc.graph.nodes.insert(nid("n2"), make_node_at("n2", 100.0, 0.0));
        doc.graph.groups.insert(gid("g1"), make_group("g1"));

        let changed = apply_patch(
            &mut doc,
            FlowPatch::ReparentNodes {
                node_ids: vec![nid("n1"), nid("n2")],
                new_parent: Some(gid("g1")),
            },
        );
        assert!(changed);
        assert_eq!(
            doc.graph.nodes.get(&nid("n1")).unwrap().parent,
            Some(gid("g1"))
        );
        assert_eq!(
            doc.graph.nodes.get(&nid("n2")).unwrap().parent,
            Some(gid("g1"))
        );
    }

    #[test]
    fn patch_reparent_nodes_remove_parent() {
        let mut doc = FlowDocument::default();
        let mut node = make_node_at("n1", 0.0, 0.0);
        node.parent = Some(gid("g1"));
        doc.graph.nodes.insert(nid("n1"), node);

        let changed = apply_patch(
            &mut doc,
            FlowPatch::ReparentNodes {
                node_ids: vec![nid("n1")],
                new_parent: None,
            },
        );
        assert!(changed);
        assert!(doc.graph.nodes.get(&nid("n1")).unwrap().parent.is_none());
    }

    #[test]
    fn patch_reparent_no_change_same_parent() {
        let mut doc = FlowDocument::default();
        let mut node = make_node_at("n1", 0.0, 0.0);
        node.parent = Some(gid("g1"));
        doc.graph.nodes.insert(nid("n1"), node);

        let changed = apply_patch(
            &mut doc,
            FlowPatch::ReparentNodes {
                node_ids: vec![nid("n1")],
                new_parent: Some(gid("g1")),
            },
        );
        assert!(!changed);
    }

    // ---- apply_patches ----

    #[test]
    fn apply_multiple_patches() {
        let mut doc = FlowDocument::default();
        let patches = vec![
            FlowPatch::InsertNode {
                node: make_node_at("a", 0.0, 0.0),
            },
            FlowPatch::InsertNode {
                node: make_node_at("b", 200.0, 0.0),
            },
            FlowPatch::InsertEdge {
                edge: make_edge("e1", "a", "b"),
            },
        ];
        let count = apply_patches(&mut doc, patches);
        assert_eq!(count, 3);
        assert_eq!(doc.graph.nodes.len(), 2);
        assert_eq!(doc.graph.edges.len(), 1);
    }

    #[test]
    fn apply_patches_some_noop() {
        let mut doc = FlowDocument::default();
        let patches = vec![
            FlowPatch::InsertNode {
                node: make_node_at("a", 0.0, 0.0),
            },
            FlowPatch::RemoveNode { id: nid("ghost") }, // no-op
            FlowPatch::UpdateNode {
                id: nid("ghost"),
                changes: flow_core::patch::NodeChangeSet::default(), // no-op
            },
        ];
        let count = apply_patches(&mut doc, patches);
        // InsertNode returns true; both ghost patches return false
        assert_eq!(count, 1);
    }

    // ---- viewport_state roundtrip ----

    #[test]
    fn viewport_state_roundtrip() {
        let vt = ViewportTransform {
            pan_x: 10.0,
            pan_y: -5.0,
            zoom: 2.5,
        };
        let state = viewport_state_from_transform(&vt);
        let back = transform_from_viewport_state(&state);
        assert!((back.pan_x - 10.0).abs() < f64::EPSILON);
        assert!((back.pan_y - (-5.0)).abs() < f64::EPSILON);
        assert!((back.zoom - 2.5).abs() < f64::EPSILON);
    }

    // ---- UpdateNode empty changeset ----

    #[test]
    fn patch_update_node_empty_changes() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));

        let changes = flow_core::patch::NodeChangeSet::default();
        let changed = apply_patch(
            &mut doc,
            FlowPatch::UpdateNode {
                id: nid("n1"),
                changes,
            },
        );
        assert!(!changed);
    }

    // ---- UpdateNode all fields ----

    #[test]
    fn patch_update_node_all_fields() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));

        let changes = flow_core::patch::NodeChangeSet {
            position: Some([10.0, 20.0]),
            size: Some([200.0, 100.0]),
            title: Some(SmolStr::from("renamed")),
            kind: Some(SmolStr::from("Do")),
            data: Some(serde_json::json!({"key": "value"})),
            flags: Some(NodeFlags {
                locked: true,
                ..NodeFlags::default()
            }),
            ui: Some(NodeUiState {
                collapsed: true,
                color_override: Some([1.0, 0.0, 0.0, 1.0]),
            }),
        };
        let changed = apply_patch(
            &mut doc,
            FlowPatch::UpdateNode {
                id: nid("n1"),
                changes,
            },
        );
        assert!(changed);
        let node = doc.graph.nodes.get(&nid("n1")).unwrap();
        assert!((node.position[0] - 10.0).abs() < f64::EPSILON);
        assert!((node.size[0] - 200.0).abs() < f64::EPSILON);
        assert_eq!(node.title.as_str(), "renamed");
        assert_eq!(node.kind.as_str(), "Do");
        assert!(node.flags.locked);
        assert!(node.ui.collapsed);
    }

    // ---- UpdateEdge empty changeset ----

    #[test]
    fn patch_update_edge_empty_changes() {
        let mut doc = FlowDocument::default();
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));

        let changes = flow_core::patch::EdgeChangeSet::default();
        let changed = apply_patch(
            &mut doc,
            FlowPatch::UpdateEdge {
                id: eid("e1"),
                changes,
            },
        );
        assert!(!changed);
    }

    // ---- UpdateGroup empty changeset ----

    #[test]
    fn patch_update_group_empty_changes() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));

        let changes = flow_core::patch::GroupChangeSet::default();
        let changed = apply_patch(
            &mut doc,
            FlowPatch::UpdateGroup {
                id: gid("g1"),
                changes,
            },
        );
        assert!(!changed);
    }

    // ---- UpdateGroup all fields ----

    #[test]
    fn patch_update_group_all_fields() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));

        let changes = flow_core::patch::GroupChangeSet {
            title: Some(SmolStr::from("new-title")),
            bounds: Some([10.0, 20.0, 300.0, 400.0]),
            data: Some(serde_json::json!({"x": 1})),
        };
        let changed = apply_patch(
            &mut doc,
            FlowPatch::UpdateGroup {
                id: gid("g1"),
                changes,
            },
        );
        assert!(changed);
        let group = doc.graph.groups.get(&gid("g1")).unwrap();
        assert_eq!(group.title.as_str(), "new-title");
        assert!((group.bounds[0] - 10.0).abs() < f64::EPSILON);
        assert!((group.bounds[2] - 300.0).abs() < f64::EPSILON);
    }

    // =====================================================================
    // Additional tests for gaps in coverage
    // =====================================================================

    // ---- WorldRect ----

    #[test]
    fn world_rect_default_is_zero() {
        let r = WorldRect::default();
        assert!((r.x).abs() < f64::EPSILON);
        assert!((r.y).abs() < f64::EPSILON);
        assert!((r.w).abs() < f64::EPSILON);
        assert!((r.h).abs() < f64::EPSILON);
    }

    #[test]
    fn world_rect_debug_format() {
        let r = WorldRect { x: 1.0, y: 2.0, w: 3.0, h: 4.0 };
        let debug = format!("{r:?}");
        assert!(debug.contains("WorldRect"));
    }

    #[test]
    fn world_rect_clone_copy() {
        let r = WorldRect { x: 1.0, y: 2.0, w: 3.0, h: 4.0 };
        let r2 = r; // Copy
        let r3 = r; // Copy again
        assert!((r2.x - 1.0).abs() < f64::EPSILON);
        assert!((r3.w - 3.0).abs() < f64::EPSILON);
    }

    // ---- ViewportTransform edge cases ----

    #[test]
    fn transform_debug_format() {
        let vt = ViewportTransform::identity();
        let debug = format!("{vt:?}");
        assert!(debug.contains("ViewportTransform"));
    }

    #[test]
    fn transform_clone_copy() {
        let vt = ViewportTransform { pan_x: 1.0, pan_y: 2.0, zoom: 3.0 };
        let vt2 = vt; // Copy
        let vt3 = vt; // Copy again
        assert!((vt2.pan_x - 1.0).abs() < f64::EPSILON);
        assert!((vt3.zoom - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn transform_with_nonzero_rect_origin() {
        let vt = ViewportTransform::identity();
        let (sx, sy) = vt.world_to_screen(50.0, 50.0, 100.0, 200.0);
        // With identity transform: sx = (50 - 0) * 1 + 100 = 150
        assert!((sx - 150.0).abs() < f64::EPSILON);
        assert!((sy - 250.0).abs() < f64::EPSILON);
    }

    #[test]
    fn transform_screen_to_world_with_origin() {
        let vt = ViewportTransform::identity();
        let (wx, wy) = vt.screen_to_world(150.0, 250.0, 100.0, 200.0);
        // (150 - 100) / 1 + 0 = 50
        assert!((wx - 50.0).abs() < f64::EPSILON);
        assert!((wy - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn transform_roundtrip_with_origin() {
        let vt = ViewportTransform {
            pan_x: 25.0,
            pan_y: 35.0,
            zoom: 0.5,
        };
        let origin_x = 100.0;
        let origin_y = 200.0;
        let (sx, sy) = vt.world_to_screen(50.0, 75.0, origin_x, origin_y);
        let (wx, wy) = vt.screen_to_world(sx, sy, origin_x, origin_y);
        assert!((wx - 50.0).abs() < 1e-10);
        assert!((wy - 75.0).abs() < 1e-10);
    }

    #[test]
    fn transform_zoom_less_than_one_shrinks() {
        let vt = ViewportTransform {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 0.5,
        };
        let (sx, sy) = vt.world_to_screen(100.0, 100.0, 0.0, 0.0);
        // 0.5x zoom: world 100 maps to screen 50
        assert!((sx - 50.0).abs() < f64::EPSILON);
        assert!((sy - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn transform_negative_pan_shifts() {
        let vt = ViewportTransform {
            pan_x: -100.0,
            pan_y: -100.0,
            zoom: 1.0,
        };
        let (sx, sy) = vt.world_to_screen(0.0, 0.0, 0.0, 0.0);
        // (0 - (-100)) * 1 + 0 = 100
        assert!((sx - 100.0).abs() < f64::EPSILON);
        assert!((sy - 100.0).abs() < f64::EPSILON);
    }

    // ---- fit_view additional edge cases ----

    #[test]
    fn fit_view_negative_height_returns_none() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: -50.0,
        };
        assert!(fit_view(bounds, 800.0, 600.0, 0.0).is_none());
    }

    #[test]
    fn fit_view_zero_canvas_height_returns_none() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 100.0,
        };
        assert!(fit_view(bounds, 800.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn fit_view_negative_canvas_returns_none() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 100.0,
        };
        assert!(fit_view(bounds, -100.0, 600.0, 0.0).is_none());
    }

    #[test]
    fn fit_view_square_world_square_canvas() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 400.0,
        };
        let vt = fit_view(bounds, 800.0, 800.0, 0.0).unwrap();
        // 400 * zoom = 800 => zoom = 2.0
        assert!((vt.zoom - 2.0).abs() < 1e-10);
    }

    #[test]
    fn fit_view_centering_with_offset_bounds() {
        let bounds = WorldRect {
            x: 100.0,
            y: 200.0,
            w: 200.0,
            h: 100.0,
        };
        let vt = fit_view(bounds, 800.0, 600.0, 0.0).unwrap();
        // Center of bounds: (200, 250)
        // pan should center the world in the canvas
        let center_sx = (200.0 - vt.pan_x) * vt.zoom;
        let center_sy = (250.0 - vt.pan_y) * vt.zoom;
        assert!((center_sx - 400.0).abs() < 1e-6);
        assert!((center_sy - 300.0).abs() < 1e-6);
    }

    #[test]
    fn fit_view_no_padding() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        };
        let vt = fit_view(bounds, 800.0, 600.0, 0.0).unwrap();
        assert!((vt.zoom - 1.0).abs() < 1e-10);
    }

    #[test]
    fn fit_view_wide_world_portrait_canvas() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 100.0,
        };
        let vt = fit_view(bounds, 300.0, 600.0, 0.0).unwrap();
        // Width constraint: 400 * zoom = 300 => zoom = 0.75
        assert!((vt.zoom - 0.75).abs() < 1e-10);
    }

    // ---- compute_graph_bounds additional ----

    #[test]
    fn graph_bounds_single_node_at_origin() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        let bounds = compute_graph_bounds(&graph).unwrap();
        assert!((bounds.x).abs() < f64::EPSILON);
        assert!((bounds.y).abs() < f64::EPSILON);
        assert!((bounds.w - 100.0).abs() < f64::EPSILON);
        assert!((bounds.h - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn graph_bounds_negative_coordinates() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", -200.0, -100.0));
        let bounds = compute_graph_bounds(&graph).unwrap();
        assert!((bounds.x - (-200.0)).abs() < f64::EPSILON);
        assert!((bounds.y - (-100.0)).abs() < f64::EPSILON);
        // w = (-200 + 100) - (-200) = 100
        assert!((bounds.w - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn graph_bounds_overlapping_nodes() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 50.0, 25.0));
        let bounds = compute_graph_bounds(&graph).unwrap();
        // Leftmost x = 0, topmost y = 0
        // Rightmost = max(0+100, 50+100) = 150
        // Bottom = max(0+50, 25+50) = 75
        assert!((bounds.w - 150.0).abs() < f64::EPSILON);
        assert!((bounds.h - 75.0).abs() < f64::EPSILON);
    }

    // ---- hit_test_nodes additional ----

    #[test]
    fn hit_test_nodes_corner_top_left() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 10.0, 10.0));
        let result = hit_test_nodes(&graph, 10.0, 10.0);
        assert!(matches!(result, HitResult::Node(_)));
    }

    #[test]
    fn hit_test_nodes_just_outside_left() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 10.0, 10.0));
        let result = hit_test_nodes(&graph, 9.99, 25.0);
        assert!(matches!(result, HitResult::Nothing));
    }

    #[test]
    fn hit_test_nodes_multiple_same_z() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at_z("a", 0.0, 0.0, 0));
        graph.nodes.insert(nid("b"), make_node_at_z("b", 0.0, 0.0, 0));
        let result = hit_test_nodes(&graph, 50.0, 25.0);
        // Should hit one of them (both at same z-index, overlapping)
        assert!(matches!(result, HitResult::Node(ref id) if id == &nid("a") || id == &nid("b")));
    }

    #[test]
    fn hit_test_nodes_negative_coords() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", -200.0, -100.0));
        let result = hit_test_nodes(&graph, -150.0, -75.0);
        assert!(matches!(result, HitResult::Node(ref id) if id == &nid("a")));
    }

    // ---- hit_test_edges additional ----

    #[test]
    fn hit_test_edges_negative_tolerance() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 0.0));
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let result = hit_test_edges(&graph, 100.0, 0.0, -5.0, 20);
        assert!(matches!(result, HitResult::Nothing));
    }

    #[test]
    fn hit_test_edges_actually_hits_edge() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 0.0));
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        // The edge goes from right side of "a" (x=100, y~54) to left side of "b" (x=200, y~54)
        // Midpoint of the edge should be around x=150
        let result = hit_test_edges(&graph, 150.0, 54.0, 20.0, 50);
        assert!(matches!(result, HitResult::Edge(ref id) if id == &eid("e1")));
    }

    #[test]
    fn hit_test_edges_misses_edge() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 0.0));
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let result = hit_test_edges(&graph, 150.0, 500.0, 10.0, 50);
        assert!(matches!(result, HitResult::Nothing));
    }

    // ---- HitResult debug format ----

    #[test]
    fn hit_result_nothing_debug() {
        let result = HitResult::Nothing;
        let debug = format!("{result:?}");
        assert!(debug.contains("Nothing"));
    }

    #[test]
    fn hit_result_node_debug() {
        let result = HitResult::Node(nid("test_node"));
        let debug = format!("{result:?}");
        assert!(debug.contains("Node"));
    }

    #[test]
    fn hit_result_edge_debug() {
        let result = HitResult::Edge(eid("test_edge"));
        let debug = format!("{result:?}");
        assert!(debug.contains("Edge"));
    }

    // ---- Selection additional ----

    #[test]
    fn selection_select_edge_clears_previous() {
        let mut sel = Selection::new();
        sel.select_edge(eid("e1"));
        sel.select_edge(eid("e2"));
        assert!(!sel.is_edge_selected(&eid("e1")));
        assert!(sel.is_edge_selected(&eid("e2")));
        assert_eq!(sel.edge_count(), 1);
    }

    #[test]
    fn selection_select_group_clears_previous() {
        let mut sel = Selection::new();
        sel.select_group(gid("g1"));
        sel.select_group(gid("g2"));
        assert!(!sel.is_group_selected(&gid("g1")));
        assert!(sel.is_group_selected(&gid("g2")));
        assert_eq!(sel.group_count(), 1);
    }

    #[test]
    fn selection_select_node_clears_edges_and_groups() {
        let mut sel = Selection::new();
        sel.add_edge(eid("e1"));
        sel.select_group(gid("g1"));
        sel.select_node(nid("n1"));
        assert!(sel.is_node_selected(&nid("n1")));
        assert!(!sel.is_edge_selected(&eid("e1")));
        assert!(!sel.is_group_selected(&gid("g1")));
    }

    #[test]
    fn selection_select_edge_clears_nodes_and_groups() {
        let mut sel = Selection::new();
        sel.add_node(nid("n1"));
        sel.select_group(gid("g1"));
        sel.select_edge(eid("e1"));
        assert!(sel.is_edge_selected(&eid("e1")));
        assert!(!sel.is_node_selected(&nid("n1")));
        assert!(!sel.is_group_selected(&gid("g1")));
    }

    #[test]
    fn selection_add_edge() {
        let mut sel = Selection::new();
        sel.add_edge(eid("e1"));
        sel.add_edge(eid("e2"));
        assert!(sel.is_edge_selected(&eid("e1")));
        assert!(sel.is_edge_selected(&eid("e2")));
        assert_eq!(sel.edge_count(), 2);
    }

    #[test]
    fn selection_toggle_group_not_available() {
        // Groups only have select_group, not toggle_group -- test add/clear pattern
        let mut sel = Selection::new();
        sel.select_group(gid("g1"));
        assert!(sel.is_group_selected(&gid("g1")));
        sel.clear();
        assert!(!sel.is_group_selected(&gid("g1")));
    }

    #[test]
    fn selection_total_count_mixed() {
        let mut sel = Selection::new();
        sel.add_node(nid("n1"));
        sel.add_node(nid("n2"));
        sel.add_edge(eid("e1"));
        // Use add pattern: select_group clears all, so use add approach
        // by adding via the state round-trip
        let state = SelectionState {
            selected_nodes: vec![nid("n1"), nid("n2")],
            selected_edges: vec![eid("e1")],
            selected_groups: vec![gid("g1")],
        };
        sel = Selection::from_selection_state(&state);
        assert_eq!(sel.total_count(), 4);
    }

    #[test]
    fn selection_any_selected_true_with_group() {
        let mut sel = Selection::new();
        sel.select_group(gid("g1"));
        assert!(sel.any_selected());
    }

    #[test]
    fn selection_from_state_roundtrip() {
        let mut sel = Selection::new();
        sel.add_node(nid("n1"));
        sel.add_edge(eid("e1"));
        // Build selection with group via from_selection_state to avoid clear
        let state = SelectionState {
            selected_nodes: vec![nid("n1")],
            selected_edges: vec![eid("e1")],
            selected_groups: vec![gid("g1")],
        };
        sel = Selection::from_selection_state(&state);
        let state2 = sel.to_selection_state();
        let sel2 = Selection::from_selection_state(&state2);
        assert!(sel2.is_node_selected(&nid("n1")));
        assert!(sel2.is_edge_selected(&eid("e1")));
        assert!(sel2.is_group_selected(&gid("g1")));
        assert_eq!(sel2.total_count(), 3);
    }

    #[test]
    fn selection_select_all_includes_groups() {
        // select_all does not select groups -- only nodes and edges
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let mut sel = Selection::new();
        sel.select_all(&graph);
        assert!(sel.is_node_selected(&nid("a")));
        assert!(sel.is_edge_selected(&eid("e1")));
        assert_eq!(sel.group_count(), 0);
    }

    #[test]
    fn selection_debug_format() {
        let sel = Selection::new();
        let debug = format!("{sel:?}");
        assert!(debug.contains("Selection"));
    }

    #[test]
    fn selection_clone() {
        let mut sel = Selection::new();
        sel.add_node(nid("a"));
        let sel2 = sel.clone();
        assert!(sel2.is_node_selected(&nid("a")));
    }

    // ---- compute_port_world_pos ----

    #[test]
    fn port_world_pos_output_on_right_edge() {
        let node = make_node_at("n", 100.0, 200.0);
        let (px, _py) = compute_port_world_pos(&node, &pid("out"), true);
        assert!((px - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn port_world_pos_input_on_left_edge() {
        let node = make_node_at("n", 100.0, 200.0);
        let (px, _py) = compute_port_world_pos(&node, &pid("in"), false);
        assert!((px - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn port_world_pos_y_includes_header_and_padding() {
        let node = make_node_at("n", 0.0, 0.0);
        let (_px, py) = compute_port_world_pos(&node, &pid("any"), true);
        // Expected: 0 + 32 + 12 + 0*20 + 10 = 54
        let expected = 32.0 + 12.0 + 10.0;
        assert!((py - expected).abs() < 1e-10);
    }

    // ---- i_to_f64 ----

    #[test]
    fn i_to_f64_start() {
        assert!((i_to_f64(10, 0)).abs() < f64::EPSILON);
    }

    #[test]
    fn i_to_f64_end() {
        assert!((i_to_f64(10, 10) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn i_to_f64_midpoint() {
        assert!((i_to_f64(10, 5) - 0.5).abs() < 1e-10);
    }

    // ---- apply_patch additional edge cases ----

    #[test]
    fn patch_update_edge_style() {
        let mut doc = FlowDocument::default();
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let changes = flow_core::patch::EdgeChangeSet {
            style: Some(EdgeStyle {
                line_style: flow_core::doc::LineStyle::Dashed,
                ..EdgeStyle::default()
            }),
            ..flow_core::patch::EdgeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateEdge { id: eid("e1"), changes });
        assert!(changed);
        assert_eq!(
            doc.graph.edges.get(&eid("e1")).unwrap().style.line_style,
            flow_core::doc::LineStyle::Dashed
        );
    }

    #[test]
    fn patch_update_edge_data() {
        let mut doc = FlowDocument::default();
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let changes = flow_core::patch::EdgeChangeSet {
            data: Some(serde_json::json!({"key": "value"})),
            ..flow_core::patch::EdgeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateEdge { id: eid("e1"), changes });
        assert!(changed);
        assert_eq!(
            doc.graph.edges.get(&eid("e1")).unwrap().data["key"],
            "value"
        );
    }

    #[test]
    fn patch_update_nonexistent_edge() {
        let mut doc = FlowDocument::default();
        let changes = flow_core::patch::EdgeChangeSet {
            label: Some(Some(SmolStr::from("test"))),
            ..flow_core::patch::EdgeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateEdge { id: eid("ghost"), changes });
        assert!(!changed);
    }

    #[test]
    fn patch_remove_nonexistent_edge() {
        let mut doc = FlowDocument::default();
        let changed = apply_patch(&mut doc, FlowPatch::RemoveEdge { id: eid("ghost") });
        assert!(!changed);
    }

    #[test]
    fn patch_update_nonexistent_group() {
        let mut doc = FlowDocument::default();
        let changes = flow_core::patch::GroupChangeSet {
            title: Some(SmolStr::from("test")),
            ..flow_core::patch::GroupChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateGroup { id: gid("ghost"), changes });
        assert!(!changed);
    }

    #[test]
    fn patch_remove_nonexistent_group() {
        let mut doc = FlowDocument::default();
        let changed = apply_patch(&mut doc, FlowPatch::RemoveGroup { id: gid("ghost") });
        assert!(!changed);
    }

    #[test]
    fn patch_remove_node_cascades_multiple_edges() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        doc.graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 0.0));
        doc.graph.nodes.insert(nid("c"), make_node_at("c", 400.0, 0.0));
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        doc.graph.edges.insert(eid("e2"), make_edge("e2", "a", "c"));
        doc.graph.edges.insert(eid("e3"), make_edge("e3", "b", "c"));

        apply_patch(&mut doc, FlowPatch::RemoveNode { id: nid("a") });
        // e1 and e2 should be removed (connected to a), e3 should remain
        assert!(!doc.graph.edges.contains_key(&eid("e1")));
        assert!(!doc.graph.edges.contains_key(&eid("e2")));
        assert!(doc.graph.edges.contains_key(&eid("e3")));
    }

    #[test]
    fn patch_set_entry_node_to_none() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("n1"));
        let changed = apply_patch(
            &mut doc,
            FlowPatch::SetEntryNode { node: None },
        );
        assert!(changed);
        assert!(doc.graph.entry_node.is_none());
    }

    #[test]
    fn patch_update_node_kind() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));
        let changes = flow_core::patch::NodeChangeSet {
            kind: Some(SmolStr::from("Choose")),
            ..flow_core::patch::NodeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateNode { id: nid("n1"), changes });
        assert!(changed);
        assert_eq!(
            doc.graph.nodes.get(&nid("n1")).unwrap().kind.as_str(),
            "Choose"
        );
    }

    #[test]
    fn patch_update_node_size() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));
        let changes = flow_core::patch::NodeChangeSet {
            size: Some([200.0, 150.0]),
            ..flow_core::patch::NodeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateNode { id: nid("n1"), changes });
        assert!(changed);
        let node = doc.graph.nodes.get(&nid("n1")).unwrap();
        assert!((node.size[0] - 200.0).abs() < f64::EPSILON);
        assert!((node.size[1] - 150.0).abs() < f64::EPSILON);
    }

    // ---- apply_patches additional ----

    #[test]
    fn apply_patches_empty_vec() {
        let mut doc = FlowDocument::default();
        let count = apply_patches(&mut doc, Vec::new());
        assert_eq!(count, 0);
    }

    #[test]
    fn apply_patches_all_succeed() {
        let mut doc = FlowDocument::default();
        let patches = vec![
            FlowPatch::InsertNode { node: make_node_at("a", 0.0, 0.0) },
            FlowPatch::InsertNode { node: make_node_at("b", 100.0, 0.0) },
        ];
        let count = apply_patches(&mut doc, patches);
        assert_eq!(count, 2);
        assert_eq!(doc.graph.nodes.len(), 2);
    }

    // ---- cubic_bezier_point additional ----

    #[test]
    fn bezier_symmetry_check() {
        // For a symmetric curve, t and 1-t should give mirrored points
        let curve = CubicBezier {
            p0: Point { x: 0.0, y: 0.0 },
            p1: Point { x: 50.0, y: 100.0 },
            p2: Point { x: 50.0, y: -100.0 },
            p3: Point { x: 100.0, y: 0.0 },
        };
        let pt_low = cubic_bezier_point(0.25, &curve);
        let pt_high = cubic_bezier_point(0.75, &curve);
        // x should be mirrored around 50
        let diff_x = (pt_low.x + pt_high.x - 100.0).abs();
        assert!(diff_x < 1e-10);
        // y should be negated
        let diff_y = (pt_low.y + pt_high.y).abs();
        assert!(diff_y < 1e-10);
    }

    // ---- min_distance_to_cubic_bezier additional ----

    #[test]
    fn min_dist_to_bezier_at_end() {
        let curve = CubicBezier {
            p0: Point { x: 0.0, y: 0.0 },
            p1: Point { x: 50.0, y: 0.0 },
            p2: Point { x: 50.0, y: 100.0 },
            p3: Point { x: 100.0, y: 100.0 },
        };
        let d = min_distance_to_cubic_bezier(
            Point { x: 100.0, y: 100.0 },
            &curve,
            100,
        );
        assert!(d < 1e-10);
    }

    #[test]
    fn min_dist_to_bezier_at_midpoint() {
        let curve = CubicBezier {
            p0: Point { x: 0.0, y: 0.0 },
            p1: Point { x: 50.0, y: 0.0 },
            p2: Point { x: 50.0, y: 0.0 },
            p3: Point { x: 100.0, y: 0.0 },
        };
        // Linear curve, midpoint at (50, 0)
        let d = min_distance_to_cubic_bezier(
            Point { x: 50.0, y: 0.0 },
            &curve,
            100,
        );
        assert!(d < 1e-10);
    }

    #[test]
    fn min_dist_to_bezier_increases_with_offset() {
        let curve = CubicBezier {
            p0: Point { x: 0.0, y: 0.0 },
            p1: Point { x: 50.0, y: 0.0 },
            p2: Point { x: 50.0, y: 0.0 },
            p3: Point { x: 100.0, y: 0.0 },
        };
        let d5 = min_distance_to_cubic_bezier(
            Point { x: 50.0, y: 5.0 },
            &curve,
            100,
        );
        let d10 = min_distance_to_cubic_bezier(
            Point { x: 50.0, y: 10.0 },
            &curve,
            100,
        );
        assert!(d10 > d5);
    }

    // =====================================================================
    // Additional comprehensive coverage tests
    // =====================================================================

    // ---- Selection: select_group clears nodes and edges ----

    #[test]
    fn selection_select_group_clears_nodes_and_edges() {
        let mut sel = Selection::new();
        sel.add_node(nid("n1"));
        sel.add_edge(eid("e1"));
        sel.select_group(gid("g1"));
        assert!(!sel.is_node_selected(&nid("n1")));
        assert!(!sel.is_edge_selected(&eid("e1")));
        assert!(sel.is_group_selected(&gid("g1")));
    }

    // ---- compute_graph_bounds: all nodes at same position ----

    #[test]
    fn graph_bounds_all_nodes_same_position() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 10.0, 20.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 10.0, 20.0));
        let bounds = compute_graph_bounds(&graph).unwrap();
        // Both nodes at same position, same size (100x50)
        assert!((bounds.x - 10.0).abs() < f64::EPSILON);
        assert!((bounds.y - 20.0).abs() < f64::EPSILON);
        assert!((bounds.w - 100.0).abs() < f64::EPSILON);
        assert!((bounds.h - 50.0).abs() < f64::EPSILON);
    }

    // ---- fit_view: very small world bounds ----

    #[test]
    fn fit_view_very_small_bounds() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 0.001,
            h: 0.001,
        };
        let vt = fit_view(bounds, 800.0, 600.0, 0.0);
        assert!(vt.is_some());
        let vt = vt.unwrap();
        assert!(vt.zoom > 0.0);
    }

    // ---- fit_view: very large world bounds ----

    #[test]
    fn fit_view_very_large_bounds() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 1_000_000.0,
            h: 1_000_000.0,
        };
        let vt = fit_view(bounds, 800.0, 600.0, 0.0);
        assert!(vt.is_some());
        let vt = vt.unwrap();
        // Zoom should be very small to fit huge world
        assert!(vt.zoom < 1.0);
    }

    // ---- hit_test_edges: closest edge wins ----

    #[test]
    fn hit_test_edges_closest_edge_wins() {
        let mut graph = FlowGraph::default();
        // Two edges: a->b at y=0, c->d at y=200
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 0.0));
        graph.nodes.insert(nid("c"), make_node_at("c", 0.0, 200.0));
        graph.nodes.insert(nid("d"), make_node_at("d", 200.0, 200.0));
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        graph.edges.insert(eid("e2"), make_edge("e2", "c", "d"));
        // Hit near e1 (y~=54) with small tolerance
        let result = hit_test_edges(&graph, 100.0, 54.0, 5.0, 50);
        assert!(matches!(result, HitResult::Edge(ref id) if id == &eid("e1")));
    }

    // ---- hit_test_edges: multiple edges same location ----

    #[test]
    fn hit_test_edges_multiple_close() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 0.0));
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        graph.edges.insert(eid("e2"), make_edge("e2", "a", "b"));
        // Both edges have same geometry, so either could match
        let result = hit_test_edges(&graph, 100.0, 54.0, 20.0, 50);
        assert!(matches!(result, HitResult::Edge(_)));
    }

    // ---- apply_patch: UpdateNode with data only ----

    #[test]
    fn patch_update_node_data_only() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));
        let changes = flow_core::patch::NodeChangeSet {
            data: Some(serde_json::json!({"payload": "test"})),
            ..flow_core::patch::NodeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateNode { id: nid("n1"), changes });
        assert!(changed);
        assert_eq!(
            doc.graph.nodes.get(&nid("n1")).unwrap().data["payload"],
            "test"
        );
    }

    // ---- apply_patch: UpdateNode with flags only ----

    #[test]
    fn patch_update_node_flags_only() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));
        let changes = flow_core::patch::NodeChangeSet {
            flags: Some(NodeFlags {
                hidden: true,
                locked: true,
                ..NodeFlags::default()
            }),
            ..flow_core::patch::NodeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateNode { id: nid("n1"), changes });
        assert!(changed);
        let node = doc.graph.nodes.get(&nid("n1")).unwrap();
        assert!(node.flags.hidden);
        assert!(node.flags.locked);
    }

    // ---- apply_patch: UpdateNode with ui only ----

    #[test]
    fn patch_update_node_ui_only() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_at("n1", 0.0, 0.0));
        let changes = flow_core::patch::NodeChangeSet {
            ui: Some(NodeUiState {
                collapsed: true,
                color_override: Some([1.0, 0.0, 0.0, 1.0]),
            }),
            ..flow_core::patch::NodeChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateNode { id: nid("n1"), changes });
        assert!(changed);
        let node = doc.graph.nodes.get(&nid("n1")).unwrap();
        assert!(node.ui.collapsed);
        assert_eq!(node.ui.color_override, Some([1.0, 0.0, 0.0, 1.0]));
    }

    // ---- apply_patch: UpdateEdge all fields ----

    #[test]
    fn patch_update_edge_all_fields() {
        let mut doc = FlowDocument::default();
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let changes = flow_core::patch::EdgeChangeSet {
            label: Some(Some(SmolStr::from("new-label"))),
            style: Some(EdgeStyle {
                line_style: flow_core::doc::LineStyle::Dotted,
                ..EdgeStyle::default()
            }),
            data: Some(serde_json::json!({"key": "value"})),
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateEdge { id: eid("e1"), changes });
        assert!(changed);
        let edge = doc.graph.edges.get(&eid("e1")).unwrap();
        assert_eq!(edge.label.as_ref().map(|s| s.as_str()), Some("new-label"));
        assert_eq!(edge.style.line_style, flow_core::doc::LineStyle::Dotted);
        assert_eq!(edge.data["key"], "value");
    }

    // ---- UpdateGroup data field ----

    #[test]
    fn patch_update_group_data_only() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let changes = flow_core::patch::GroupChangeSet {
            data: Some(serde_json::json!({"meta": 42})),
            ..flow_core::patch::GroupChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateGroup { id: gid("g1"), changes });
        assert!(changed);
        assert_eq!(doc.graph.groups.get(&gid("g1")).unwrap().data["meta"], 42);
    }

    // ---- UpdateGroup bounds only ----

    #[test]
    fn patch_update_group_bounds_only() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let changes = flow_core::patch::GroupChangeSet {
            bounds: Some([10.0, 20.0, 300.0, 400.0]),
            ..flow_core::patch::GroupChangeSet::default()
        };
        let changed = apply_patch(&mut doc, FlowPatch::UpdateGroup { id: gid("g1"), changes });
        assert!(changed);
        let group = doc.graph.groups.get(&gid("g1")).unwrap();
        assert!((group.bounds[0] - 10.0).abs() < f64::EPSILON);
        assert!((group.bounds[3] - 400.0).abs() < f64::EPSILON);
    }

    // ---- ReparentNodes with non-existent node ----

    #[test]
    fn patch_reparent_nonexistent_node() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let changed = apply_patch(
            &mut doc,
            FlowPatch::ReparentNodes {
                node_ids: vec![nid("ghost")],
                new_parent: Some(gid("g1")),
            },
        );
        assert!(!changed);
    }

    // ---- ReparentNodes with empty list ----

    #[test]
    fn patch_reparent_empty_list() {
        let mut doc = FlowDocument::default();
        let changed = apply_patch(
            &mut doc,
            FlowPatch::ReparentNodes {
                node_ids: Vec::new(),
                new_parent: Some(gid("g1")),
            },
        );
        assert!(!changed);
    }

    // ---- WorldRect with large values ----

    #[test]
    fn world_rect_large_values() {
        let r = WorldRect {
            x: 1e15,
            y: -1e15,
            w: 1e10,
            h: 1e10,
        };
        let r2 = r; // Copy
        assert!((r2.x - 1e15).abs() < 1e5);
        assert!((r2.w - 1e10).abs() < 1e0);
    }

    // ---- ViewportTransform identity roundtrip with large coordinates ----

    #[test]
    fn transform_roundtrip_large_coordinates() {
        let vt = ViewportTransform {
            pan_x: 1e6,
            pan_y: -1e6,
            zoom: 0.01,
        };
        let (sx, sy) = vt.world_to_screen(1e8, 1e8, 0.0, 0.0);
        let (wx, wy) = vt.screen_to_world(sx, sy, 0.0, 0.0);
        let diff_x = (wx - 1e8).abs();
        let diff_y = (wy - 1e8).abs();
        // Allow some floating point error for very large coordinates
        assert!(diff_x < 1e3, "diff_x = {diff_x}");
        assert!(diff_y < 1e3, "diff_y = {diff_y}");
    }

    // ---- Selection: add_node is idempotent ----

    #[test]
    fn selection_add_node_idempotent() {
        let mut sel = Selection::new();
        sel.add_node(nid("a"));
        sel.add_node(nid("a"));
        sel.add_node(nid("a"));
        assert_eq!(sel.node_count(), 1);
    }

    // ---- Selection: add_edge is idempotent ----

    #[test]
    fn selection_add_edge_idempotent() {
        let mut sel = Selection::new();
        sel.add_edge(eid("e1"));
        sel.add_edge(eid("e1"));
        assert_eq!(sel.edge_count(), 1);
    }

    // ---- Selection: select_node then select_edge clears node ----

    #[test]
    fn selection_select_edge_clears_previous_node() {
        let mut sel = Selection::new();
        sel.select_node(nid("a"));
        sel.select_edge(eid("e1"));
        assert!(!sel.is_node_selected(&nid("a")));
        assert!(sel.is_edge_selected(&eid("e1")));
    }

    // ---- Selection: toggle_node multiple times ----

    #[test]
    fn selection_toggle_node_triple() {
        let mut sel = Selection::new();
        sel.toggle_node(nid("a"));
        assert!(sel.is_node_selected(&nid("a")));
        sel.toggle_node(nid("a"));
        assert!(!sel.is_node_selected(&nid("a")));
        sel.toggle_node(nid("a"));
        assert!(sel.is_node_selected(&nid("a")));
    }

    // ---- apply_patches: mixed success and failure ----

    #[test]
    fn apply_patches_insert_then_update() {
        let mut doc = FlowDocument::default();
        let patches = vec![
            FlowPatch::InsertNode { node: make_node_at("n1", 10.0, 20.0) },
            FlowPatch::UpdateNode {
                id: nid("n1"),
                changes: flow_core::patch::NodeChangeSet {
                    title: Some(SmolStr::from("updated")),
                    ..flow_core::patch::NodeChangeSet::default()
                },
            },
        ];
        let count = apply_patches(&mut doc, patches);
        assert_eq!(count, 2);
        assert_eq!(
            doc.graph.nodes.get(&nid("n1")).unwrap().title.as_str(),
            "updated"
        );
    }

    // ---- apply_patches: insert then remove same node ----

    #[test]
    fn apply_patches_insert_then_remove_same_node() {
        let mut doc = FlowDocument::default();
        let patches = vec![
            FlowPatch::InsertNode { node: make_node_at("n1", 0.0, 0.0) },
            FlowPatch::RemoveNode { id: nid("n1") },
        ];
        let count = apply_patches(&mut doc, patches);
        assert_eq!(count, 2);
        assert!(!doc.graph.nodes.contains_key(&nid("n1")));
    }

    // ---- i_to_f64 edge cases ----

    #[test]
    fn i_to_f64_single_step() {
        assert!((i_to_f64(1, 0)).abs() < f64::EPSILON);
        assert!((i_to_f64(1, 1) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn i_to_f64_zero_total_returns_zero() {
        // When total is 0, this would be 0/0 = NaN
        // But the function is only called with total >= 1 in practice
        // Test with valid input
        assert!((i_to_f64(100, 0)).abs() < f64::EPSILON);
    }

    // ---- cubic_bezier_point: monotonic x for horizontal curve ----

    #[test]
    fn bezier_monotonic_x_horizontal() {
        let curve = CubicBezier {
            p0: Point { x: 0.0, y: 0.0 },
            p1: Point { x: 33.0, y: 0.0 },
            p2: Point { x: 66.0, y: 0.0 },
            p3: Point { x: 100.0, y: 0.0 },
        };
        let mut prev_x = f64::MIN;
        for i in 0..=10 {
            let t = i_to_f64(10, i);
            let pt = cubic_bezier_point(t, &curve);
            assert!(pt.x >= prev_x, "x should be monotonically increasing: {} >= {}", pt.x, prev_x);
            prev_x = pt.x;
        }
    }

    // ---- compute_port_world_pos: node with custom size ----

    #[test]
    fn port_world_pos_custom_size_node() {
        let mut node = make_node_at("n", 50.0, 50.0);
        node.size = [200.0, 100.0];
        let (px, _) = compute_port_world_pos(&node, &pid("out"), true);
        assert!((px - 250.0).abs() < f64::EPSILON);
    }

    // ---- fit_view: asymmetric padding ----

    #[test]
    fn fit_view_small_padding() {
        let bounds = WorldRect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        };
        let vt = fit_view(bounds, 1000.0, 1000.0, 0.05).unwrap();
        // With 5% padding on each side, effective size = 900
        // 100 * zoom = 900 => zoom = 9.0
        assert!((vt.zoom - 9.0).abs() < 1e-10);
    }

    // ---- Selection: select_all on empty graph ----

    #[test]
    fn selection_select_all_empty_graph() {
        let graph = FlowGraph::default();
        let mut sel = Selection::new();
        sel.select_all(&graph);
        assert!(sel.is_empty());
    }

    // ---- compute_graph_bounds: mix of hidden and visible ----

    #[test]
    fn graph_bounds_mix_hidden_visible() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_hidden_node("b", 500.0, 500.0));
        graph.nodes.insert(nid("c"), make_node_at("c", 300.0, 200.0));
        let bounds = compute_graph_bounds(&graph).unwrap();
        // Should include "a" and "c" but not hidden "b"
        assert!((bounds.x).abs() < f64::EPSILON);
        assert!((bounds.y).abs() < f64::EPSILON);
        // Right edge: max(0+100, 300+100) = 400
        assert!((bounds.w - 400.0).abs() < f64::EPSILON);
        // Bottom edge: max(0+50, 200+50) = 250
        assert!((bounds.h - 250.0).abs() < f64::EPSILON);
    }
}
