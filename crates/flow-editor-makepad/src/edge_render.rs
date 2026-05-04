//! Edge rendering: bezier curves, arrow heads, animated particles, and
//! kind-specific styling for the flow-editor canvas.
//!
//! This module is a pure-logic layer (no Makepad imports) so every function
//! can be unit-tested in isolation. The caller (FlowEditor widget) feeds
//! `FlowEdgeRecord`s and a `ViewportTransform` into `EdgeRenderer`, which
//! produces `EdgeRenderData` containing the curve geometry, style, and
//! particle positions ready for GPU drawing.

use flow_core::doc::{EdgeMarker, FlowEdgeRecord, FlowGraph, FlowNodeRecord, LineStyle};
use flow_core::ids::PortId;
use std::time::Duration;

use crate::draw;
use crate::theme;

// ---------------------------------------------------------------------------
// Geometry types
// ---------------------------------------------------------------------------

/// A 2D point in world or screen coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Euclidean distance to another point.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn distance_to(&self, other: Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

impl Default for Point {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// Four control points of a cubic bezier curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    pub p0: Point,
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
}

impl CubicBezier {
    pub const fn new(p0: Point, p1: Point, p2: Point, p3: Point) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// Evaluate the cubic bezier at parameter `t` in [0, 1].
    #[allow(clippy::arithmetic_side_effects)]
    pub fn evaluate(&self, t: f64) -> Point {
        let u = 1.0 - t;
        let uu = u * u;
        let uuu = uu * u;
        let tt = t * t;
        let ttt = tt * t;

        Point {
            x: uuu * self.p0.x
                + 3.0 * uu * t * self.p1.x
                + 3.0 * u * tt * self.p2.x
                + ttt * self.p3.x,
            y: uuu * self.p0.y
                + 3.0 * uu * t * self.p1.y
                + 3.0 * u * tt * self.p2.y
                + ttt * self.p3.y,
        }
    }

    /// Evaluate the tangent (first derivative) at parameter `t`.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn tangent(&self, t: f64) -> Point {
        let u = 1.0 - t;
        let uu = u * u;
        let tt = t * t;

        Point {
            x: 3.0 * uu * (self.p1.x - self.p0.x)
                + 6.0 * u * t * (self.p2.x - self.p1.x)
                + 3.0 * tt * (self.p3.x - self.p2.x),
            y: 3.0 * uu * (self.p1.y - self.p0.y)
                + 6.0 * u * t * (self.p2.y - self.p1.y)
                + 3.0 * tt * (self.p3.y - self.p2.y),
        }
    }

    /// Approximate arc length by sampling at `num_segments + 1` points.
    #[allow(clippy::arithmetic_side_effects, clippy::as_conversions)]
    pub fn arc_length(&self, num_segments: usize) -> f64 {
        if num_segments == 0 {
            return 0.0;
        }
        let denom = num_segments as f64;
        let mut length = 0.0;
        let mut prev = self.evaluate(0.0);
        for i in 1..=num_segments {
            let t = i as f64 / denom;
            let curr = self.evaluate(t);
            length += prev.distance_to(curr);
            prev = curr;
        }
        length
    }
}

// ---------------------------------------------------------------------------
// Edge style classification
// ---------------------------------------------------------------------------

/// Kind-specific edge visual style, derived from `LineStyle` and edge metadata.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EdgeDashStyle {
    Solid,
    Dashed,
    Dotted,
}

/// Resolved visual style for a single edge.
#[derive(Clone, Copy, Debug)]
pub struct EdgeVisualStyle {
    pub color: [f32; 4],
    pub width: f32,
    pub dash: EdgeDashStyle,
    pub animated: bool,
    pub marker: EdgeMarker,
}

/// Resolve the visual style for an edge from its record.
pub fn resolve_edge_style(edge: &FlowEdgeRecord) -> EdgeVisualStyle {
    let color = resolve_edge_color(edge);
    let dash = match edge.style.line_style {
        LineStyle::Solid => EdgeDashStyle::Solid,
        LineStyle::Dashed => EdgeDashStyle::Dashed,
        LineStyle::Dotted => EdgeDashStyle::Dotted,
    };

    EdgeVisualStyle {
        color,
        width: edge.style.width,
        dash,
        animated: edge.style.animated,
        marker: edge.style.marker,
    }
}

/// Resolve the color for an edge based on its line style and any color override.
pub fn resolve_edge_color(edge: &FlowEdgeRecord) -> [f32; 4] {
    if let Some(color) = edge.ui.color_override {
        return color;
    }
    match edge.style.line_style {
        LineStyle::Solid => theme::colors::NEON_CYAN,
        LineStyle::Dashed => theme::colors::STATE_FAILED,
        LineStyle::Dotted => theme::colors::STATE_ASKING,
    }
}

// ---------------------------------------------------------------------------
// Control point computation
// ---------------------------------------------------------------------------

/// Compute the four control points of a cubic bezier from source to target
/// port world positions. The curve exits the source horizontally to the
/// right and enters the target horizontally from the left.
#[allow(clippy::arithmetic_side_effects)]
pub fn compute_control_points(source: Point, target: Point) -> CubicBezier {
    let dx = (target.x - source.x).abs();
    let cp_offset = dx.max(draw::edge::BEZIER_CP_MIN) * draw::edge::BEZIER_CP_FRACTION;

    CubicBezier {
        p0: source,
        p1: Point::new(source.x + cp_offset, source.y),
        p2: Point::new(target.x - cp_offset, target.y),
        p3: target,
    }
}

/// Compute the world-space position of a port on a node.
/// If `is_output`, uses the right side; otherwise the left side.
#[allow(clippy::arithmetic_side_effects)]
pub fn compute_port_world_pos(
    node: &FlowNodeRecord,
    port_id: &PortId,
    is_output: bool,
) -> Point {
    let header_h: f64 = draw::node::HEADER_HEIGHT;
    let padding: f64 = draw::node::PADDING;
    let port_height: f64 = draw::port::HEIGHT;

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

    Point::new(px, py)
}

// ---------------------------------------------------------------------------
// Arrow head geometry
// ---------------------------------------------------------------------------

/// Three vertices that form a triangular arrow head.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowHead {
    pub tip: Point,
    pub left: Point,
    pub right: Point,
}

impl ArrowHead {
    /// Compute an arrow head at `position` pointing in the direction of
    /// `tangent`, with the given `size` (length from base to tip).
    #[allow(clippy::arithmetic_side_effects)]
    pub fn from_tangent(tip: Point, tangent: Point, size: f64) -> Option<Self> {
        let len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
        if len < 1e-12 {
            return None;
        }

        // Normalize the tangent direction.
        let dx = tangent.x / len;
        let dy = tangent.y / len;

        // Perpendicular direction (rotated 90 degrees).
        let perp_x = -dy;
        let perp_y = dx;

        // Half-width of the arrow base, ~0.4 * size gives a reasonable angle.
        let half_width = size * 0.4;

        // Base center is behind the tip by `size` in the tangent direction.
        let base_x = tip.x - dx * size;
        let base_y = tip.y - dy * size;

        Some(ArrowHead {
            tip,
            left: Point::new(base_x + perp_x * half_width, base_y + perp_y * half_width),
            right: Point::new(base_x - perp_x * half_width, base_y - perp_y * half_width),
        })
    }
}

/// Compute the arrow head for an edge at the target end of the bezier curve.
/// Returns `None` if the tangent is degenerate.
pub fn compute_arrow_head(curve: &CubicBezier, size: f64) -> Option<ArrowHead> {
    let tangent = curve.tangent(1.0);
    ArrowHead::from_tangent(curve.p3, tangent, size)
}

/// The default arrow head size in world units.
pub const ARROW_HEAD_SIZE: f64 = 10.0;

// ---------------------------------------------------------------------------
// Animated particle
// ---------------------------------------------------------------------------

/// A single particle that travels along a bezier curve.
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    /// Parametric position along the curve [0, 1].
    pub t: f64,
    /// World-space position.
    pub position: Point,
}

/// Compute particle positions along a bezier curve for a given animation time.
///
/// `elapsed` is the total animation time. `speed` is in world units per second.
/// `count` is how many particles to place evenly along the curve.
/// `num_arc_samples` controls the accuracy of arc-length parameterization.
#[allow(clippy::arithmetic_side_effects)]
pub fn compute_particles(
    curve: &CubicBezier,
    elapsed: Duration,
    speed: f64,
    count: usize,
    num_arc_samples: usize,
) -> Vec<Particle> {
    if count == 0 || speed <= 0.0 {
        return Vec::new();
    }

    let arc_len = curve.arc_length(num_arc_samples.max(16));
    if arc_len < 1e-12 {
        return Vec::new();
    }

    // How long one full traversal takes.
    let traversal_secs = arc_len / speed;
    let traversal_secs = if traversal_secs < 1e-12 {
        1.0
    } else {
        traversal_secs
    };

    let elapsed_secs = elapsed.as_secs_f64();

    // Fractional progress of the first particle.
    let progress = (elapsed_secs % traversal_secs) / traversal_secs;

    #[allow(clippy::as_conversions)]
    let count_f64 = count as f64;
    let mut particles = Vec::with_capacity(count);
    for i in 0..count {
        // Evenly space particles, offset by the animation progress.
        let spacing = 1.0 / count_f64;
        #[allow(clippy::as_conversions)]
        let offset = i as f64 * spacing;
        let t = (progress + offset) % 1.0;
        let position = curve.evaluate(t);
        particles.push(Particle { t, position });
    }

    particles
}

// ---------------------------------------------------------------------------
// EdgeRenderer
// ---------------------------------------------------------------------------

/// Render data for a single edge, ready for GPU drawing.
#[derive(Clone, Debug)]
pub struct EdgeRenderData {
    /// Unique edge identifier (for selection / hit testing).
    pub edge_id: flow_core::ids::EdgeId,
    /// The cubic bezier curve in world coordinates.
    pub curve: CubicBezier,
    /// Resolved visual style.
    pub style: EdgeVisualStyle,
    /// Arrow head at the target end, if applicable.
    pub arrow_head: Option<ArrowHead>,
    /// Animated particles along the curve.
    pub particles: Vec<Particle>,
}

/// Produces `EdgeRenderData` from a `FlowGraph`.
pub struct EdgeRenderer {
    /// Arrow head size in world units.
    pub arrow_size: f64,
    /// Particle speed in world units per second.
    pub particle_speed: f64,
    /// Number of particles per animated edge.
    pub particle_count: usize,
    /// Number of arc-length samples for particle parameterization.
    pub arc_samples: usize,
}

impl EdgeRenderer {
    /// Create with default settings.
    pub fn new() -> Self {
        Self {
            arrow_size: ARROW_HEAD_SIZE,
            particle_speed: draw::edge::PARTICLE_SPEED,
            particle_count: 3,
            arc_samples: 64,
        }
    }

    /// Render all visible edges in the graph.
    pub fn render_edges(
        &self,
        graph: &FlowGraph,
        elapsed: Duration,
    ) -> Vec<EdgeRenderData> {
        let mut results = Vec::new();
        for edge in graph.edges.values() {
            if let Some(data) = self.render_edge(graph, edge, elapsed) {
                results.push(data);
            }
        }
        results
    }

    /// Render a single edge. Returns `None` if source or target node is missing.
    fn render_edge(
        &self,
        graph: &FlowGraph,
        edge: &FlowEdgeRecord,
        elapsed: Duration,
    ) -> Option<EdgeRenderData> {
        let source = graph.nodes.get(&edge.source_node)?;
        let target = graph.nodes.get(&edge.target_node)?;

        let src_pos = compute_port_world_pos(source, &edge.source_port, true);
        let tgt_pos = compute_port_world_pos(target, &edge.target_port, false);

        let curve = compute_control_points(src_pos, tgt_pos);
        let style = resolve_edge_style(edge);

        let arrow_head = match style.marker {
            EdgeMarker::None => None,
            EdgeMarker::Arrow | EdgeMarker::ArrowFilled => {
                compute_arrow_head(&curve, self.arrow_size)
            }
            EdgeMarker::Circle => None,
        };

        let particles = if style.animated {
            compute_particles(
                &curve,
                elapsed,
                self.particle_speed,
                self.particle_count,
                self.arc_samples,
            )
        } else {
            Vec::new()
        };

        Some(EdgeRenderData {
            edge_id: edge.id.clone(),
            curve,
            style,
            arrow_head,
            particles,
        })
    }
}

impl Default for EdgeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use flow_core::doc::{
        EdgeStyle, FlowEdgeRecord, FlowGraph, FlowNodeRecord, NodeFlags, NodeUiState,
    };
    use flow_core::ids::PortId;
    use smol_str::SmolStr;

    // ---- helpers ----

    fn nid(s: &str) -> flow_core::ids::NodeId {
        SmolStr::from(s)
    }

    fn eid(s: &str) -> flow_core::ids::EdgeId {
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
            size: [100.0, 80.0],
            z_index: 0,
            parent: None,
            ports: Vec::new(),
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
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

    fn make_edge_with_style(id: &str, src: &str, tgt: &str, style: EdgeStyle) -> FlowEdgeRecord {
        FlowEdgeRecord {
            id: eid(id),
            source_node: nid(src),
            source_port: pid("out"),
            target_node: nid(tgt),
            target_port: pid("in"),
            label: None,
            style,
            data: serde_json::Value::Null,
            ui: flow_core::doc::EdgeUiState::default(),
        }
    }

    fn make_animated_edge(id: &str, src: &str, tgt: &str) -> FlowEdgeRecord {
        let mut style = EdgeStyle::default();
        style.animated = true;
        make_edge_with_style(id, src, tgt, style)
    }

    fn build_two_node_graph() -> FlowGraph {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 0.0));
        graph
    }

    // ======================================================================
    // Point tests
    // ======================================================================

    #[test]
    fn point_default_is_origin() {
        let p = Point::default();
        assert!((p.x).abs() < f64::EPSILON);
        assert!((p.y).abs() < f64::EPSILON);
    }

    #[test]
    fn point_new() {
        let p = Point::new(3.0, 4.0);
        assert!((p.x - 3.0).abs() < f64::EPSILON);
        assert!((p.y - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn point_distance_to_self() {
        let p = Point::new(5.0, 10.0);
        assert!((p.distance_to(p)).abs() < 1e-12);
    }

    #[test]
    fn point_distance_horizontal() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(10.0, 0.0);
        assert!((a.distance_to(b) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn point_distance_345_triangle() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 4.0);
        assert!((a.distance_to(b) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn point_equality() {
        assert_eq!(Point::new(1.0, 2.0), Point::new(1.0, 2.0));
        assert_ne!(Point::new(1.0, 2.0), Point::new(1.0, 3.0));
    }

    // ======================================================================
    // CubicBezier tests
    // ======================================================================

    #[test]
    fn bezier_evaluate_at_zero_is_start() {
        let curve = CubicBezier::new(
            Point::new(1.0, 2.0),
            Point::new(10.0, 20.0),
            Point::new(30.0, 40.0),
            Point::new(50.0, 60.0),
        );
        let pt = curve.evaluate(0.0);
        assert!((pt.x - 1.0).abs() < 1e-10);
        assert!((pt.y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn bezier_evaluate_at_one_is_end() {
        let curve = CubicBezier::new(
            Point::new(1.0, 2.0),
            Point::new(10.0, 20.0),
            Point::new(30.0, 40.0),
            Point::new(50.0, 60.0),
        );
        let pt = curve.evaluate(1.0);
        assert!((pt.x - 50.0).abs() < 1e-10);
        assert!((pt.y - 60.0).abs() < 1e-10);
    }

    #[test]
    fn bezier_linear_midpoint() {
        // When control points are at the midpoint of a straight line,
        // t=0.5 should be the exact midpoint.
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(50.0, 50.0),
            Point::new(50.0, 50.0),
            Point::new(100.0, 100.0),
        );
        let pt = curve.evaluate(0.5);
        assert!((pt.x - 50.0).abs() < 1e-10);
        assert!((pt.y - 50.0).abs() < 1e-10);
    }

    #[test]
    fn bezier_tangent_at_zero_is_initial_direction() {
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(200.0, 0.0),
            Point::new(300.0, 0.0),
        );
        let tan = curve.tangent(0.0);
        // Tangent at t=0 is 3*(p1 - p0) = (300, 0)
        assert!((tan.x - 300.0).abs() < 1e-10);
        assert!((tan.y).abs() < 1e-10);
    }

    #[test]
    fn bezier_arc_length_straight_line() {
        // A straight line from (0,0) to (100,0) should have arc length 100.
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(33.33, 0.0),
            Point::new(66.66, 0.0),
            Point::new(100.0, 0.0),
        );
        let len = curve.arc_length(100);
        assert!((len - 100.0).abs() < 1.0);
    }

    #[test]
    fn bezier_arc_length_zero_segments() {
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(20.0, 0.0),
            Point::new(30.0, 0.0),
        );
        assert!((curve.arc_length(0)).abs() < f64::EPSILON);
    }

    #[test]
    fn bezier_equality() {
        let a = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(3.0, 3.0),
        );
        let b = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(3.0, 3.0),
        );
        assert_eq!(a, b);
    }

    // ======================================================================
    // Control point computation tests
    // ======================================================================

    #[test]
    fn control_points_source_is_p0() {
        let src = Point::new(0.0, 50.0);
        let tgt = Point::new(200.0, 50.0);
        let curve = compute_control_points(src, tgt);
        assert_eq!(curve.p0, src);
    }

    #[test]
    fn control_points_target_is_p3() {
        let src = Point::new(0.0, 50.0);
        let tgt = Point::new(200.0, 50.0);
        let curve = compute_control_points(src, tgt);
        assert_eq!(curve.p3, tgt);
    }

    #[test]
    fn control_points_cp1_exits_horizontally() {
        let src = Point::new(0.0, 50.0);
        let tgt = Point::new(200.0, 100.0);
        let curve = compute_control_points(src, tgt);
        // CP1 should have the same y as source (horizontal exit).
        assert!((curve.p1.y - src.y).abs() < 1e-10);
        // CP1 should be to the right of source.
        assert!(curve.p1.x > src.x);
    }

    #[test]
    fn control_points_cp2_enters_horizontally() {
        let src = Point::new(0.0, 50.0);
        let tgt = Point::new(200.0, 100.0);
        let curve = compute_control_points(src, tgt);
        // CP2 should have the same y as target (horizontal entry).
        assert!((curve.p2.y - tgt.y).abs() < 1e-10);
        // CP2 should be to the left of target.
        assert!(curve.p2.x < tgt.x);
    }

    #[test]
    fn control_points_close_nodes_use_minimum_offset() {
        let src = Point::new(0.0, 50.0);
        let tgt = Point::new(5.0, 50.0);
        let curve = compute_control_points(src, tgt);
        // Even with tiny dx, control points should be offset by at least
        // BEZIER_CP_MIN * BEZIER_CP_FRACTION = 40 * 0.4 = 16.0
        let min_offset = draw::edge::BEZIER_CP_MIN * draw::edge::BEZIER_CP_FRACTION;
        assert!(curve.p1.x - src.x >= min_offset - 1e-10);
        assert!(tgt.x - curve.p2.x >= min_offset - 1e-10);
    }

    // ======================================================================
    // Port position tests
    // ======================================================================

    #[test]
    fn port_world_pos_output_is_right_edge() {
        let node = make_node_at("n", 100.0, 200.0);
        let pos = compute_port_world_pos(&node, &pid("any"), true);
        assert!((pos.x - 200.0).abs() < 1e-10); // 100 + 100 (size)
    }

    #[test]
    fn port_world_pos_input_is_left_edge() {
        let node = make_node_at("n", 100.0, 200.0);
        let pos = compute_port_world_pos(&node, &pid("any"), false);
        assert!((pos.x - 100.0).abs() < 1e-10);
    }

    #[test]
    fn port_world_pos_y_includes_header_and_padding() {
        let node = make_node_at("n", 0.0, 0.0);
        let pos = compute_port_world_pos(&node, &pid("any"), true);
        // Expected y: 0 + 32 (header) + 12 (padding) + 0 (order=0) * 20 + 10 (half port height) = 54
        let expected_y = draw::node::HEADER_HEIGHT + draw::node::PADDING + draw::port::HEIGHT / 2.0;
        assert!((pos.y - expected_y).abs() < 1e-10);
    }

    // ======================================================================
    // Arrow head tests
    // ======================================================================

    #[test]
    fn arrow_head_horizontal_right() {
        let tip = Point::new(100.0, 50.0);
        let tangent = Point::new(1.0, 0.0);
        let arrow = ArrowHead::from_tangent(tip, tangent, 10.0).unwrap();
        assert_eq!(arrow.tip, tip);
        // Left and right should be behind and above/below the tip.
        assert!(arrow.left.x < tip.x);
        assert!(arrow.right.x < tip.x);
        assert!(arrow.left.y > tip.y);
        assert!(arrow.right.y < tip.y);
    }

    #[test]
    fn arrow_head_vertical_down() {
        let tip = Point::new(50.0, 100.0);
        let tangent = Point::new(0.0, 1.0);
        let arrow = ArrowHead::from_tangent(tip, tangent, 10.0).unwrap();
        assert_eq!(arrow.tip, tip);
        assert!(arrow.left.y < tip.y);
        assert!(arrow.right.y < tip.y);
    }

    #[test]
    fn arrow_head_zero_tangent_returns_none() {
        let tip = Point::new(50.0, 50.0);
        let tangent = Point::new(0.0, 0.0);
        assert!(ArrowHead::from_tangent(tip, tangent, 10.0).is_none());
    }

    #[test]
    fn arrow_head_from_curve() {
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(100.0, 0.0),
        );
        let arrow = compute_arrow_head(&curve, 10.0).unwrap();
        // Tip should be at the end of the curve.
        assert!((arrow.tip.x - 100.0).abs() < 1e-10);
        assert!((arrow.tip.y).abs() < 1e-10);
    }

    // ======================================================================
    // Edge style resolution tests
    // ======================================================================

    #[test]
    fn solid_edge_gets_cyan_color() {
        let edge = make_edge("e1", "a", "b");
        let color = resolve_edge_color(&edge);
        assert_eq!(color, theme::colors::NEON_CYAN);
    }

    #[test]
    fn dashed_edge_gets_red_color() {
        let mut style = EdgeStyle::default();
        style.line_style = LineStyle::Dashed;
        let edge = make_edge_with_style("e1", "a", "b", style);
        let color = resolve_edge_color(&edge);
        assert_eq!(color, theme::colors::STATE_FAILED);
    }

    #[test]
    fn dotted_edge_gets_yellow_color() {
        let mut style = EdgeStyle::default();
        style.line_style = LineStyle::Dotted;
        let edge = make_edge_with_style("e1", "a", "b", style);
        let color = resolve_edge_color(&edge);
        assert_eq!(color, theme::colors::STATE_ASKING);
    }

    #[test]
    fn color_override_takes_priority() {
        let override_color = [1.0, 0.5, 0.0, 1.0];
        let mut edge = make_edge("e1", "a", "b");
        edge.ui.color_override = Some(override_color);
        let color = resolve_edge_color(&edge);
        assert_eq!(color, override_color);
    }

    #[test]
    fn resolve_style_returns_correct_dash() {
        let edge = make_edge("e1", "a", "b");
        let style = resolve_edge_style(&edge);
        assert_eq!(style.dash, EdgeDashStyle::Solid);
        assert!(!style.animated);
        assert_eq!(style.marker, EdgeMarker::Arrow);
    }

    // ======================================================================
    // Particle tests
    // ======================================================================

    #[test]
    fn particles_zero_count_returns_empty() {
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(100.0, 0.0),
        );
        let particles = compute_particles(&curve, Duration::from_millis(500), 50.0, 0, 64);
        assert!(particles.is_empty());
    }

    #[test]
    fn particles_zero_speed_returns_empty() {
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(100.0, 0.0),
        );
        let particles = compute_particles(&curve, Duration::from_millis(500), 0.0, 3, 64);
        assert!(particles.is_empty());
    }

    #[test]
    fn particles_returns_correct_count() {
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(100.0, 0.0),
        );
        let particles = compute_particles(&curve, Duration::from_millis(500), 50.0, 5, 64);
        assert_eq!(particles.len(), 5);
    }

    #[test]
    fn particles_positions_are_on_curve() {
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(100.0, 0.0),
        );
        let particles = compute_particles(&curve, Duration::from_millis(100), 50.0, 3, 64);
        for p in &particles {
            let expected = curve.evaluate(p.t);
            assert!((p.position.x - expected.x).abs() < 1e-10);
            assert!((p.position.y - expected.y).abs() < 1e-10);
        }
    }

    #[test]
    fn particles_t_values_in_range() {
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(100.0, 0.0),
        );
        let particles = compute_particles(&curve, Duration::from_millis(9999), 50.0, 4, 64);
        for p in &particles {
            assert!(p.t >= 0.0 && p.t < 1.0);
        }
    }

    // ======================================================================
    // EdgeRenderer integration tests
    // ======================================================================

    #[test]
    fn renderer_empty_graph_returns_empty() {
        let graph = FlowGraph::default();
        let renderer = EdgeRenderer::new();
        let results = renderer.render_edges(&graph, Duration::ZERO);
        assert!(results.is_empty());
    }

    #[test]
    fn renderer_edge_with_missing_nodes_is_skipped() {
        let mut graph = FlowGraph::default();
        // Edge references non-existent nodes.
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let renderer = EdgeRenderer::new();
        let results = renderer.render_edges(&graph, Duration::ZERO);
        assert!(results.is_empty());
    }

    #[test]
    fn renderer_produces_valid_edge() {
        let mut graph = build_two_node_graph();
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));

        let renderer = EdgeRenderer::new();
        let results = renderer.render_edges(&graph, Duration::ZERO);
        assert_eq!(results.len(), 1);

        let data = &results[0];
        assert_eq!(data.edge_id, eid("e1"));
        // Curve should go from right side of "a" to left side of "b".
        assert!((data.curve.p0.x - 100.0).abs() < 1e-10); // a right edge
        assert!((data.curve.p3.x - 200.0).abs() < 1e-10); // b left edge
    }

    #[test]
    fn renderer_arrow_head_present_by_default() {
        let mut graph = build_two_node_graph();
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));

        let renderer = EdgeRenderer::new();
        let results = renderer.render_edges(&graph, Duration::ZERO);
        assert_eq!(results.len(), 1);
        assert!(results[0].arrow_head.is_some());
    }

    #[test]
    fn renderer_no_marker_means_no_arrow() {
        let mut graph = build_two_node_graph();
        let mut style = EdgeStyle::default();
        style.marker = EdgeMarker::None;
        graph.edges.insert(eid("e1"), make_edge_with_style("e1", "a", "b", style));

        let renderer = EdgeRenderer::new();
        let results = renderer.render_edges(&graph, Duration::ZERO);
        assert!(results[0].arrow_head.is_none());
    }

    #[test]
    fn renderer_animated_edge_has_particles() {
        let mut graph = build_two_node_graph();
        graph.edges.insert(eid("e1"), make_animated_edge("e1", "a", "b"));

        let renderer = EdgeRenderer::new();
        let results = renderer.render_edges(&graph, Duration::from_millis(500));
        assert_eq!(results.len(), 1);
        assert!(!results[0].particles.is_empty());
    }

    #[test]
    fn renderer_non_animated_edge_has_no_particles() {
        let mut graph = build_two_node_graph();
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));

        let renderer = EdgeRenderer::new();
        let results = renderer.render_edges(&graph, Duration::from_millis(500));
        assert!(results[0].particles.is_empty());
    }

    #[test]
    fn renderer_multiple_edges() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(nid("a"), make_node_at("a", 0.0, 0.0));
        graph.nodes.insert(nid("b"), make_node_at("b", 200.0, 0.0));
        graph.nodes.insert(nid("c"), make_node_at("c", 400.0, 0.0));
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        graph.edges.insert(eid("e2"), make_edge("e2", "b", "c"));

        let renderer = EdgeRenderer::new();
        let results = renderer.render_edges(&graph, Duration::ZERO);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn renderer_default_matches_new() {
        let new_renderer = EdgeRenderer::new();
        let default_renderer = EdgeRenderer::default();
        assert!((new_renderer.arrow_size - default_renderer.arrow_size).abs() < f64::EPSILON);
        assert!((new_renderer.particle_speed - default_renderer.particle_speed).abs() < f64::EPSILON);
        assert_eq!(new_renderer.particle_count, default_renderer.particle_count);
        assert_eq!(new_renderer.arc_samples, default_renderer.arc_samples);
    }

    #[test]
    fn renderer_circle_marker_no_arrow() {
        let mut graph = build_two_node_graph();
        let mut style = EdgeStyle::default();
        style.marker = EdgeMarker::Circle;
        graph.edges.insert(eid("e1"), make_edge_with_style("e1", "a", "b", style));

        let renderer = EdgeRenderer::new();
        let results = renderer.render_edges(&graph, Duration::ZERO);
        assert!(results[0].arrow_head.is_none());
    }

    // =====================================================================
    // Additional comprehensive coverage tests
    // =====================================================================

    // ---- EdgeDashStyle derive tests ----

    #[test]
    fn edge_dash_style_equality() {
        assert_eq!(EdgeDashStyle::Solid, EdgeDashStyle::Solid);
        assert_eq!(EdgeDashStyle::Dashed, EdgeDashStyle::Dashed);
        assert_eq!(EdgeDashStyle::Dotted, EdgeDashStyle::Dotted);
        assert_ne!(EdgeDashStyle::Solid, EdgeDashStyle::Dashed);
        assert_ne!(EdgeDashStyle::Dashed, EdgeDashStyle::Dotted);
    }

    #[test]
    fn edge_dash_style_debug_format() {
        let debug = format!("{:?}", EdgeDashStyle::Dotted);
        assert!(debug.contains("Dotted"));
    }

    #[test]
    fn edge_dash_style_clone_copy() {
        let s1 = EdgeDashStyle::Dashed;
        let s2 = s1; // Copy
        assert_eq!(s1, s2);
    }

    // ---- EdgeVisualStyle derive tests ----

    #[test]
    fn edge_visual_style_debug_format() {
        let style = EdgeVisualStyle {
            color: theme::colors::NEON_CYAN,
            width: 2.0,
            dash: EdgeDashStyle::Solid,
            animated: false,
            marker: EdgeMarker::Arrow,
        };
        let debug = format!("{style:?}");
        assert!(debug.contains("EdgeVisualStyle"));
    }

    #[test]
    fn edge_visual_style_clone() {
        let style = EdgeVisualStyle {
            color: theme::colors::NEON_CYAN,
            width: 3.0,
            dash: EdgeDashStyle::Dashed,
            animated: true,
            marker: EdgeMarker::ArrowFilled,
        };
        let cloned = style.clone();
        let diff_width = (cloned.width - style.width).abs();
        assert!(diff_width < f32::EPSILON);
        assert_eq!(cloned.dash, style.dash);
        assert_eq!(cloned.animated, style.animated);
    }

    // ---- EdgeRenderData derive tests ----

    #[test]
    fn edge_render_data_debug_format() {
        let mut graph = build_two_node_graph();
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let renderer = EdgeRenderer::new();
        let results = renderer.render_edges(&graph, Duration::ZERO);
        let debug = format!("{:?}", results[0]);
        assert!(debug.contains("EdgeRenderData"));
    }

    #[test]
    fn edge_render_data_clone() {
        let mut graph = build_two_node_graph();
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let renderer = EdgeRenderer::new();
        let results = renderer.render_edges(&graph, Duration::ZERO);
        let cloned = results[0].clone();
        assert_eq!(cloned.edge_id, results[0].edge_id);
    }

    // ---- ARROW_HEAD_SIZE constant ----

    #[test]
    fn arrow_head_size_is_positive() {
        assert!(ARROW_HEAD_SIZE > 0.0);
    }

    #[test]
    fn arrow_head_size_is_reasonable() {
        // Should be larger than particle size but smaller than a typical node
        assert!(ARROW_HEAD_SIZE > draw::edge::PARTICLE_SIZE);
        assert!(ARROW_HEAD_SIZE < draw::node::MIN_WIDTH);
    }

    // ---- Point derive tests ----

    #[test]
    fn point_clone_copy() {
        let p1 = Point::new(5.0, 10.0);
        let p2 = p1; // Copy
        let p3 = p1; // Copy again
        assert_eq!(p2, p3);
    }

    // ---- CubicBezier derive tests ----

    #[test]
    fn cubic_bezier_clone_copy() {
        let c1 = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 2.0),
            Point::new(3.0, 4.0),
            Point::new(5.0, 6.0),
        );
        let c2 = c1; // Copy
        assert_eq!(c1, c2);
    }

    #[test]
    fn cubic_bezier_debug_format() {
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(3.0, 3.0),
        );
        let debug = format!("{curve:?}");
        assert!(debug.contains("CubicBezier"));
    }

    // ---- ArrowHead derive tests ----

    #[test]
    fn arrow_head_equality() {
        let ah1 = ArrowHead {
            tip: Point::new(10.0, 20.0),
            left: Point::new(5.0, 25.0),
            right: Point::new(5.0, 15.0),
        };
        let ah2 = ArrowHead {
            tip: Point::new(10.0, 20.0),
            left: Point::new(5.0, 25.0),
            right: Point::new(5.0, 15.0),
        };
        assert_eq!(ah1, ah2);
    }

    #[test]
    fn arrow_head_inequality() {
        let ah1 = ArrowHead {
            tip: Point::new(10.0, 20.0),
            left: Point::new(5.0, 25.0),
            right: Point::new(5.0, 15.0),
        };
        let ah2 = ArrowHead {
            tip: Point::new(11.0, 20.0),
            left: Point::new(5.0, 25.0),
            right: Point::new(5.0, 15.0),
        };
        assert_ne!(ah1, ah2);
    }

    #[test]
    fn arrow_head_debug_format() {
        let ah = ArrowHead {
            tip: Point::new(10.0, 20.0),
            left: Point::new(5.0, 25.0),
            right: Point::new(5.0, 15.0),
        };
        let debug = format!("{ah:?}");
        assert!(debug.contains("ArrowHead"));
    }

    // ---- Arrow head with diagonal tangent ----

    #[test]
    fn arrow_head_diagonal_tangent() {
        let tip = Point::new(100.0, 100.0);
        let tangent = Point::new(1.0, 1.0);
        let arrow = ArrowHead::from_tangent(tip, tangent, 10.0).unwrap();
        assert_eq!(arrow.tip, tip);
        // Both left and right should be behind the tip (closer to origin)
        assert!(arrow.left.x < tip.x || arrow.left.y < tip.y);
        assert!(arrow.right.x < tip.x || arrow.right.y < tip.y);
    }

    // ---- Arrow head with negative coordinates ----

    #[test]
    fn arrow_head_negative_coordinates() {
        let tip = Point::new(-50.0, -50.0);
        let tangent = Point::new(1.0, 0.0);
        let arrow = ArrowHead::from_tangent(tip, tangent, 10.0).unwrap();
        assert_eq!(arrow.tip, tip);
        // Base center is behind the tip (tip - direction*size), so left/right.x < tip.x.
        assert!(arrow.left.x < tip.x);
        assert!(arrow.right.x < tip.x);
    }

    // ---- EdgeRenderer custom configuration ----

    #[test]
    fn renderer_custom_arrow_size() {
        let mut renderer = EdgeRenderer::new();
        renderer.arrow_size = 20.0;
        let mut graph = build_two_node_graph();
        graph.edges.insert(eid("e1"), make_edge("e1", "a", "b"));
        let results = renderer.render_edges(&graph, Duration::ZERO);
        // Arrow should be computed with the custom size (20.0)
        let arrow = results[0].arrow_head.unwrap();
        // Distance from tip to base center should be proportional to size
        let dist = arrow.tip.distance_to(
            Point::new(
                (arrow.left.x + arrow.right.x) / 2.0,
                (arrow.left.y + arrow.right.y) / 2.0,
            ),
        );
        assert!((dist - 20.0).abs() < 1.0);
    }

    #[test]
    fn renderer_custom_particle_count() {
        let mut renderer = EdgeRenderer::new();
        renderer.particle_count = 7;
        let mut graph = build_two_node_graph();
        graph.edges.insert(eid("e1"), make_animated_edge("e1", "a", "b"));
        let results = renderer.render_edges(&graph, Duration::from_millis(100));
        assert_eq!(results[0].particles.len(), 7);
    }

    // ---- resolve_edge_style with dashed ----

    #[test]
    fn resolve_style_dashed_edge() {
        let mut style = EdgeStyle::default();
        style.line_style = LineStyle::Dashed;
        let edge = make_edge_with_style("e1", "a", "b", style);
        let resolved = resolve_edge_style(&edge);
        assert_eq!(resolved.dash, EdgeDashStyle::Dashed);
        assert_eq!(resolved.color, theme::colors::STATE_FAILED);
    }

    // ---- resolve_edge_style with dotted ----

    #[test]
    fn resolve_style_dotted_edge() {
        let mut style = EdgeStyle::default();
        style.line_style = LineStyle::Dotted;
        let edge = make_edge_with_style("e1", "a", "b", style);
        let resolved = resolve_edge_style(&edge);
        assert_eq!(resolved.dash, EdgeDashStyle::Dotted);
        assert_eq!(resolved.color, theme::colors::STATE_ASKING);
    }

    // ---- Particle debug format ----

    #[test]
    fn particle_debug_format() {
        let particle = Particle {
            t: 0.5,
            position: Point::new(50.0, 50.0),
        };
        let debug = format!("{particle:?}");
        assert!(debug.contains("Particle"));
    }

    // ---- Particle clone ----

    #[test]
    fn particle_clone() {
        let particle = Particle {
            t: 0.3,
            position: Point::new(10.0, 20.0),
        };
        let cloned = particle.clone();
        let diff_t = (cloned.t - particle.t).abs();
        assert!(diff_t < f64::EPSILON);
        assert_eq!(cloned.position, particle.position);
    }

    // ---- Bezier tangent at end ----

    #[test]
    fn bezier_tangent_at_one_is_final_direction() {
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(300.0, 0.0),
        );
        let tan = curve.tangent(1.0);
        // Tangent at t=1 is 3*(p3 - p2) = (900, 0)
        assert!((tan.x - 900.0).abs() < 1e-10);
        assert!(tan.y.abs() < 1e-10);
    }

    // ---- Bezier arc_length with many segments ----

    #[test]
    fn bezier_arc_length_converges() {
        let curve = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(50.0, 100.0),
            Point::new(50.0, -100.0),
            Point::new(100.0, 0.0),
        );
        let len_16 = curve.arc_length(16);
        let len_64 = curve.arc_length(64);
        let len_256 = curve.arc_length(256);
        // More samples should give more accurate (and converging) results
        assert!(len_64 > 0.0);
        assert!(len_256 > 0.0);
        // Higher resolution should be close to lower resolution
        let diff = (len_256 - len_64).abs();
        assert!(diff < (len_64 * 0.1), "arc length should converge: diff={diff}");
    }

    // ---- compute_port_world_pos with default port order ----

    #[test]
    fn port_world_pos_missing_port_uses_order_zero() {
        let node = make_node_at("n", 0.0, 0.0);
        // "nonexistent" port is not in the node's port list, so order defaults to 0
        let pos = compute_port_world_pos(&node, &pid("nonexistent"), true);
        // Should use order=0, so y should be header + padding + port_height/2
        let expected_y = 32.0 + 12.0 + 10.0;
        assert!((pos.y - expected_y).abs() < 1e-10);
    }

    // ---- Control points for backward edge (source right of target) ----

    #[test]
    fn control_points_backward_edge() {
        let src = Point::new(200.0, 50.0);
        let tgt = Point::new(0.0, 50.0);
        let curve = compute_control_points(src, tgt);
        assert_eq!(curve.p0, src);
        assert_eq!(curve.p3, tgt);
        // CP1 exits horizontally from source
        assert!((curve.p1.y - src.y).abs() < 1e-10);
        // CP2 enters horizontally to target
        assert!((curve.p2.y - tgt.y).abs() < 1e-10);
    }
}
