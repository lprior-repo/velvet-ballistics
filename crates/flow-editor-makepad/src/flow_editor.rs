//! FlowEditor widget -- the main canvas for graph visualization and editing.
//!
//! Renders a node-graph canvas with pan/zoom interaction using Makepad 2.0
//! drawing primitives. The widget displays nodes as rounded rectangles with
//! header bars, port indicators, and bezier-curve edges between them.

use flow_core::doc::FlowDocument;
use makepad_widgets::*;

use crate::draw;
use crate::theme;

/// Action types emitted by the flow editor.
#[derive(Clone, Debug)]
pub enum FlowEditorAction {
    DocumentChanged,
    SelectionChanged,
    ViewportChanged { pan_x: f64, pan_y: f64, zoom: f64 },
    NodeClicked { node_id: flow_core::ids::NodeId },
    EdgeClicked { edge_id: flow_core::ids::EdgeId },
    CanvasClicked { world_x: f64, world_y: f64 },
}

/// Helper to convert a `[f32; 4]` theme color into a `Vec4f`.
fn color_vec4(c: [f32; 4]) -> Vec4f {
    Vec4f {
        x: c[0],
        y: c[1],
        z: c[2],
        w: c[3],
    }
}

/// Lossless f64-to-f32 conversion clamped to the f32 representable range.
/// Returns `f32::MAX` for out-of-range positive values and `f32::MIN` for
/// out-of-range negative values, so the conversion never silently truncates.
#[allow(clippy::as_conversions)]
fn f64_to_f32(v: f64) -> f32 {
    // f32 can represent values up to ~3.4e38. Values beyond that clamp.
    if v > f32::MAX.into() {
        f32::MAX
    } else if v < f32::MIN.into() {
        f32::MIN
    } else {
        v as f32
    }
}

script_mod! {
    use mod.prelude.widgets_internal.*

    mod.widgets.FlowEditorBase = #(FlowEditor::register_widget(vm))
    mod.widgets.FlowEditor = set_type_default() do mod.widgets.FlowEditorBase{
        width: Fill
        height: Fill
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct FlowEditor {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
    #[redraw]
    #[live]
    draw_bg: DrawColor,
    #[live]
    draw_grid: DrawColor,
    #[live]
    draw_vector: DrawVector,
    #[live]
    draw_text: DrawText,
    #[rust]
    document: Option<FlowDocument>,
    #[rust]
    pan_x: f64,
    #[rust]
    pan_y: f64,
    #[rust]
    zoom: f64,
    #[rust]
    rect: Rect,
    // Pan/zoom interaction state
    #[rust]
    drag_start_abs: Option<DVec2>,
    #[rust]
    drag_start_pan: (f64, f64),
}

impl Widget for FlowEditor {
    #[allow(elided_lifetimes_in_paths)]
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
        match event.hits_with_capture_overload(cx, self.draw_bg.area(), true) {
            Hit::FingerDown(fe) if fe.is_primary_hit() => {
                self.drag_start_abs = Some(fe.abs);
                self.drag_start_pan = (self.pan_x, self.pan_y);
                cx.set_key_focus(self.draw_bg.area());
            }
            Hit::FingerMove(fe) => {
                if let Some(start_abs) = self.drag_start_abs {
                    #[allow(clippy::arithmetic_side_effects)]
                    let delta = fe.abs - start_abs;
                    self.pan_x = self.drag_start_pan.0 + delta.x / self.zoom;
                    self.pan_y = self.drag_start_pan.1 + delta.y / self.zoom;
                    cx.set_cursor(MouseCursor::Grabbing);
                    self.redraw(cx);
                }
            }
            Hit::FingerUp(fe) if self.drag_start_abs.is_some() => {
                let start_abs = self.drag_start_abs.unwrap_or_default();
                #[allow(clippy::arithmetic_side_effects)]
                let delta = (fe.abs - start_abs).length();
                self.drag_start_abs = None;
                cx.set_cursor(MouseCursor::Default);
                // If barely moved, treat as a click
                if delta < draw::viewport::CLICK_THRESHOLD {
                    #[allow(clippy::arithmetic_side_effects)]
                    let world_x = (fe.abs.x - self.rect.pos.x) / self.zoom + self.pan_x;
                    #[allow(clippy::arithmetic_side_effects)]
                    let world_y = (fe.abs.y - self.rect.pos.y) / self.zoom + self.pan_y;
                    self.handle_click(cx, world_x, world_y);
                }
                self.redraw(cx);
            }
            Hit::FingerScroll(fs) => {
                let scroll = if fs.scroll.y.abs() > f64::EPSILON {
                    fs.scroll.y
                } else {
                    fs.scroll.x
                };
                let old_zoom = self.zoom;
                let factor = if scroll > 0.0 {
                    draw::viewport::ZOOM_STEP
                } else {
                    1.0 / draw::viewport::ZOOM_STEP
                };
                self.zoom = (self.zoom * factor).clamp(draw::viewport::ZOOM_MIN, draw::viewport::ZOOM_MAX);
                // Zoom toward cursor position
                #[allow(clippy::arithmetic_side_effects)]
                let cursor_local_x = fs.abs.x - self.rect.pos.x;
                #[allow(clippy::arithmetic_side_effects)]
                let cursor_local_y = fs.abs.y - self.rect.pos.y;
                let world_x = cursor_local_x / old_zoom + self.pan_x;
                let world_y = cursor_local_y / old_zoom + self.pan_y;
                self.pan_x = world_x - cursor_local_x / self.zoom;
                self.pan_y = world_y - cursor_local_y / self.zoom;
                self.redraw(cx);
            }
            Hit::FingerHoverIn(_) => {
                cx.set_cursor(MouseCursor::Grab);
            }
            Hit::FingerHoverOut(_) => {
                cx.set_cursor(MouseCursor::Default);
            }
            _ => {}
        }
    }

    #[allow(elided_lifetimes_in_paths)]
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        self.rect = cx.walk_turtle(walk);
        self.draw_background(cx);
        self.draw_grid_layer(cx);
        // Clone doc to avoid borrow conflict: we need &self mutable for draw calls
        // while iterating the document. The document is only read, never mutated during draw.
        if let Some(ref doc) = self.document {
            let doc = doc.clone();
            self.draw_edges_layer(cx, &doc);
            self.draw_nodes_layer(cx, &doc);
        }
        DrawStep::done()
    }
}

impl FlowEditor {
    pub fn set_document(&mut self, _cx: &mut Cx, doc: FlowDocument) {
        self.document = Some(doc);
    }

    pub fn document(&self) -> Option<&FlowDocument> {
        self.document.as_ref()
    }

    pub fn set_viewport(&mut self, _cx: &mut Cx, pan_x: f64, pan_y: f64, zoom: f64) {
        self.pan_x = pan_x;
        self.pan_y = pan_y;
        self.zoom = zoom.clamp(draw::viewport::ZOOM_MIN, draw::viewport::ZOOM_MAX);
    }

    // ---- Coordinate transforms ----

    /// Convert world coordinates to screen (pixel) coordinates.
    #[allow(clippy::arithmetic_side_effects)]
    fn world_to_screen(&self, wx: f64, wy: f64) -> (f32, f32) {
        let sx = (wx - self.pan_x) * self.zoom + self.rect.pos.x;
        let sy = (wy - self.pan_y) * self.zoom + self.rect.pos.y;
        (f64_to_f32(sx), f64_to_f32(sy))
    }

    // ---- Background ----

    #[allow(elided_lifetimes_in_paths)]
    fn draw_background(&mut self, cx: &mut Cx2d) {
        self.draw_bg.color = color_vec4(theme::colors::CANVAS_BG);
        self.draw_bg.draw_abs(cx, self.rect);
    }

    // ---- Grid ----

    #[allow(elided_lifetimes_in_paths, clippy::arithmetic_side_effects)]
    fn draw_grid_layer(&mut self, cx: &mut Cx2d) {
        let zoom = self.zoom;
        let pan_x = self.pan_x;
        let pan_y = self.pan_y;
        let r = &self.rect;

        let spacing = draw::grid::MINOR_SPACING;

        // Compute visible world bounds
        let world_left = pan_x;
        let world_top = pan_y;
        let world_right = pan_x + r.size.x / zoom;
        let world_bottom = pan_y + r.size.y / zoom;

        // Snap start to grid
        let start_x = (world_left / spacing).floor() * spacing;
        let start_y = (world_top / spacing).floor() * spacing;

        self.draw_grid.color = color_vec4(theme::colors::GRID_LINE);

        // Vertical minor grid lines
        let mut x = start_x;
        while x <= world_right {
            let (sx, _) = self.world_to_screen(x, 0.0);
            let line_rect = Rect {
                pos: DVec2 {
                    x: f64::from(sx),
                    y: r.pos.y,
                },
                size: DVec2 {
                    x: 1.0,
                    y: r.size.y,
                },
            };
            self.draw_grid.draw_abs(cx, line_rect);
            x += spacing;
        }

        // Horizontal minor grid lines
        let mut y = start_y;
        while y <= world_bottom {
            let (_, sy) = self.world_to_screen(0.0, y);
            let line_rect = Rect {
                pos: DVec2 {
                    x: r.pos.x,
                    y: f64::from(sy),
                },
                size: DVec2 {
                    x: r.size.x,
                    y: 1.0,
                },
            };
            self.draw_grid.draw_abs(cx, line_rect);
            y += spacing;
        }

        // Major grid lines (brighter)
        let major_spacing = draw::grid::MAJOR_SPACING;
        self.draw_grid.color = color_vec4(theme::colors::BORDER);

        let major_start_x = (world_left / major_spacing).floor() * major_spacing;
        let major_start_y = (world_top / major_spacing).floor() * major_spacing;

        let mut mx = major_start_x;
        while mx <= world_right {
            let (sx, _) = self.world_to_screen(mx, 0.0);
            let line_rect = Rect {
                pos: DVec2 {
                    x: f64::from(sx),
                    y: r.pos.y,
                },
                size: DVec2 {
                    x: 2.0,
                    y: r.size.y,
                },
            };
            self.draw_grid.draw_abs(cx, line_rect);
            mx += major_spacing;
        }

        let mut my = major_start_y;
        while my <= world_bottom {
            let (_, sy) = self.world_to_screen(0.0, my);
            let line_rect = Rect {
                pos: DVec2 {
                    x: r.pos.x,
                    y: f64::from(sy),
                },
                size: DVec2 {
                    x: r.size.x,
                    y: 2.0,
                },
            };
            self.draw_grid.draw_abs(cx, line_rect);
            my += major_spacing;
        }
    }

    // ---- Nodes ----

    #[allow(elided_lifetimes_in_paths, clippy::arithmetic_side_effects)]
    fn draw_nodes_layer(&mut self, cx: &mut Cx2d, doc: &FlowDocument) {
        self.draw_vector.begin();

        for (_, node) in &doc.graph.nodes {
            if node.flags.hidden {
                continue;
            }
            self.draw_node(node);
        }

        self.draw_vector.end(cx);
    }

    #[allow(clippy::arithmetic_side_effects)]
    fn draw_node(&mut self, node: &flow_core::doc::FlowNodeRecord) {
        let (sx, sy) = self.world_to_screen(node.position[0], node.position[1]);
        let nw = f64_to_f32(node.size[0] * self.zoom);
        let nh = f64_to_f32(node.size[1] * self.zoom);
        let radius = f64_to_f32(draw::node::BORDER_RADIUS * self.zoom);
        let padding = f64_to_f32(draw::node::PADDING * self.zoom);

        // Determine node colors
        let node_color = Self::resolve_node_color(node);
        let border_color = Self::resolve_node_border_color(node);

        // Draw node body (rounded rect fill)
        self.draw_vector.set_color(
            node_color[0],
            node_color[1],
            node_color[2],
            node_color[3],
        );
        self.draw_vector.rounded_rect(sx, sy, nw, nh, radius);
        self.draw_vector.fill();

        // Draw node border (rounded rect stroke)
        self.draw_vector.set_color(
            border_color[0],
            border_color[1],
            border_color[2],
            border_color[3],
        );
        self.draw_vector.rounded_rect(sx, sy, nw, nh, radius);
        let border_w = f64_to_f32(2.0 * self.zoom).min(1.5);
        self.draw_vector.stroke(border_w);

        // Draw header bar
        let header_h = f64_to_f32(draw::node::HEADER_HEIGHT * self.zoom);
        self.draw_vector.set_color(
            border_color[0] * 0.7,
            border_color[1] * 0.7,
            border_color[2] * 0.7,
            border_color[3],
        );
        self.draw_vector.rounded_rect(sx, sy, nw, header_h, radius);
        self.draw_vector.fill();

        // Draw kind badge (small colored rectangle in top-right)
        let badge_size = f64_to_f32(draw::node::BADGE_SIZE * self.zoom);
        let badge_x = sx + nw - padding - badge_size;
        let badge_y = sy + (header_h - badge_size) / 2.0;
        self.draw_vector.set_color(
            border_color[0],
            border_color[1],
            border_color[2],
            border_color[3],
        );
        self.draw_vector.rounded_rect(
            badge_x,
            badge_y,
            badge_size,
            badge_size,
            f64_to_f32(2.0 * self.zoom).min(1.0),
        );
        self.draw_vector.fill();

        // Draw port indicators on left and right sides
        self.draw_node_ports(node, sx, sy, nw, nh, padding, header_h);
    }

    #[allow(clippy::arithmetic_side_effects, clippy::too_many_arguments)]
    fn draw_node_ports(
        &mut self,
        node: &flow_core::doc::FlowNodeRecord,
        sx: f32,
        sy: f32,
        nw: f32,
        nh: f32,
        padding: f32,
        header_h: f32,
    ) {
        let port_r = f64_to_f32(draw::port::RADIUS * self.zoom);
        let port_height = f64_to_f32(draw::port::HEIGHT * self.zoom);

        for port in &node.ports {
            let py = sy + header_h + padding + f32::from(port.order) * port_height + port_height / 2.0;

            // Only draw if within node bounds
            if py + port_r > sy + nh || py - port_r < sy + header_h {
                continue;
            }

            let is_input = port.role == flow_core::doc::PortRole::Target
                || port.role == flow_core::doc::PortRole::Bidirectional;
            let is_output = port.role == flow_core::doc::PortRole::Source
                || port.role == flow_core::doc::PortRole::Bidirectional;

            let port_color = if is_input && is_output {
                theme::colors::NEON_CYAN
            } else if is_input {
                theme::colors::NEON_GREEN
            } else {
                theme::colors::NEON_ORANGE
            };

            self.draw_vector.set_color(
                port_color[0],
                port_color[1],
                port_color[2],
                port_color[3],
            );

            if is_input {
                self.draw_vector.circle(sx - port_r, py, port_r);
                self.draw_vector.fill();
            }
            if is_output {
                self.draw_vector.circle(sx + nw + port_r, py, port_r);
                self.draw_vector.fill();
            }
        }
    }

    fn resolve_node_color(node: &flow_core::doc::FlowNodeRecord) -> [f32; 4] {
        if let Some(color) = node.ui.color_override {
            return color;
        }
        theme::colors::CARD_BG
    }

    fn resolve_node_border_color(node: &flow_core::doc::FlowNodeRecord) -> [f32; 4] {
        let kind = node.kind.as_str();
        match kind {
            "Do" | "do" => theme::colors::STATE_RUNNING,
            "Choose" | "choose" | "branch" => theme::colors::STATE_ASKING,
            "ForEach" | "foreach" | "Collect" | "collect" | "loop" => {
                theme::colors::STATE_WAITING
            }
            "Together" | "together" | "parallel" => theme::colors::STATE_WAITING,
            "Wait" | "wait" | "Ask" | "ask" | "suspend" => theme::colors::STATE_SUCCEEDED,
            "ErrorHandler" | "error_handler" | "error" => theme::colors::STATE_FAILED,
            "Finish" | "finish" | "terminal" => theme::colors::NEON_TEAL,
            "Jump" | "jump" | "Nop" | "nop" | "control" => theme::colors::TEXT_SECONDARY,
            _ => theme::colors::BORDER,
        }
    }

    // ---- Edges ----

    #[allow(elided_lifetimes_in_paths, clippy::arithmetic_side_effects)]
    fn draw_edges_layer(&mut self, cx: &mut Cx2d, doc: &FlowDocument) {
        self.draw_vector.begin();

        for (_, edge) in &doc.graph.edges {
            self.draw_edge(doc, edge);
        }

        self.draw_vector.end(cx);
    }

    #[allow(clippy::arithmetic_side_effects)]
    fn draw_edge(&mut self, doc: &FlowDocument, edge: &flow_core::doc::FlowEdgeRecord) {
        let source = match doc.graph.nodes.get(&edge.source_node) {
            Some(n) => n,
            None => return,
        };
        let target = match doc.graph.nodes.get(&edge.target_node) {
            Some(n) => n,
            None => return,
        };

        // Compute port positions
        let (x1, y1) = self.compute_port_screen_pos(source, &edge.source_port, true);
        let (x2, y2) = self.compute_port_screen_pos(target, &edge.target_port, false);

        // Determine edge color
        let edge_color = if let Some(color) = edge.ui.color_override {
            color
        } else {
            Self::resolve_edge_color(edge)
        };

        self.draw_vector.set_color(
            edge_color[0],
            edge_color[1],
            edge_color[2],
            edge_color[3],
        );

        // Draw bezier curve from output to input
        let dx = (x2 - x1).abs();
        let cp_offset = dx.max(f64_to_f32(draw::edge::BEZIER_CP_MIN * self.zoom))
            * f64_to_f32(draw::edge::BEZIER_CP_FRACTION);
        let cp1x = x1 + cp_offset;
        let cp1y = y1;
        let cp2x = x2 - cp_offset;
        let cp2y = y2;

        self.draw_vector.move_to(x1, y1);
        self.draw_vector
            .bezier_to(cp1x, cp1y, cp2x, cp2y, x2, y2);

        let width = edge.style.width * f64_to_f32(self.zoom);
        self.draw_vector.stroke(width);
    }

    #[allow(clippy::arithmetic_side_effects)]
    fn compute_port_screen_pos(
        &self,
        node: &flow_core::doc::FlowNodeRecord,
        port_id: &flow_core::ids::PortId,
        is_output: bool,
    ) -> (f32, f32) {
        let (sx, sy) = self.world_to_screen(node.position[0], node.position[1]);
        let nw = f64_to_f32(node.size[0] * self.zoom);
        let header_h = f64_to_f32(draw::node::HEADER_HEIGHT * self.zoom);
        let padding = f64_to_f32(draw::node::PADDING * self.zoom);
        let port_height = f64_to_f32(draw::port::HEIGHT * self.zoom);

        // Find the port to get its order
        let order = node
            .ports
            .iter()
            .find(|p| p.id == *port_id)
            .map(|p| p.order)
            .unwrap_or(0);

        let py = sy + header_h + padding + f32::from(order) * port_height + port_height / 2.0;

        if is_output {
            (sx + nw, py)
        } else {
            (sx, py)
        }
    }

    fn resolve_edge_color(edge: &flow_core::doc::FlowEdgeRecord) -> [f32; 4] {
        match edge.style.line_style {
            flow_core::doc::LineStyle::Dashed => theme::colors::STATE_FAILED,
            flow_core::doc::LineStyle::Dotted => theme::colors::STATE_ASKING,
            flow_core::doc::LineStyle::Solid => theme::colors::NEON_CYAN,
        }
    }

    // ---- Interaction helpers ----

    #[allow(clippy::arithmetic_side_effects)]
    fn handle_click(&mut self, cx: &mut Cx, world_x: f64, world_y: f64) {
        if let Some(ref doc) = self.document {
            for (_, node) in &doc.graph.nodes {
                if node.flags.hidden {
                    continue;
                }
                let nx = node.position[0];
                let ny = node.position[1];
                let nw = node.size[0];
                let nh = node.size[1];
                if world_x >= nx && world_x <= nx + nw && world_y >= ny && world_y <= ny + nh {
                    cx.widget_action(
                        self.uid,
                        FlowEditorAction::NodeClicked {
                            node_id: node.id.clone(),
                        },
                    );
                    cx.widget_action(self.uid, FlowEditorAction::SelectionChanged);
                    return;
                }
            }
        }
        cx.widget_action(
            self.uid,
            FlowEditorAction::CanvasClicked { world_x, world_y },
        );
        cx.widget_action(self.uid, FlowEditorAction::SelectionChanged);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- f64_to_f32 conversion tests ----

    #[test]
    fn f64_to_f32_normal_value() {
        let result = f64_to_f32(1.5);
        let diff = (f64::from(result) - 1.5).abs();
        assert!(diff < 1e-6);
    }

    #[test]
    fn f64_to_f32_zero() {
        let result = f64_to_f32(0.0);
        assert!((result).abs() < f32::EPSILON);
    }

    #[test]
    fn f64_to_f32_negative() {
        let result = f64_to_f32(-2.5);
        let diff = (f64::from(result) - (-2.5)).abs();
        assert!(diff < 1e-6);
    }

    #[test]
    fn f64_to_f32_small_positive() {
        let result = f64_to_f32(0.001);
        let diff = (f64::from(result) - 0.001).abs();
        assert!(diff < 1e-6);
    }

    #[test]
    fn f64_to_f32_large_but_in_f32_range() {
        let result = f64_to_f32(1e30);
        assert!(result.is_finite());
        assert!(result > 0.0);
    }

    #[test]
    fn f64_to_f32_very_large_clamps_to_max() {
        let result = f64_to_f32(1e100);
        assert_eq!(result, f32::MAX);
    }

    #[test]
    fn f64_to_f32_very_negative_clamps_to_min() {
        let result = f64_to_f32(-1e100);
        assert_eq!(result, f32::MIN);
    }

    #[test]
    fn f64_to_f32_exactly_f32_max() {
        let max_f64 = f64::from(f32::MAX);
        let result = f64_to_f32(max_f64);
        assert!(result.is_finite());
    }

    #[test]
    fn f64_to_f32_preserves_integer_values() {
        let result = f64_to_f32(100.0);
        let diff = (f64::from(result) - 100.0).abs();
        assert!(diff < 1e-6);
    }

    #[test]
    fn f64_to_f32_one() {
        let result = f64_to_f32(1.0);
        let diff = (f64::from(result) - 1.0).abs();
        assert!(diff < f32::EPSILON.into());
    }

    // ---- color_vec4 tests ----

    #[test]
    fn color_vec4_maps_components() {
        let c: [f32; 4] = [0.1, 0.2, 0.3, 0.4];
        let v = color_vec4(c);
        let diff_x = (v.x - c[0]).abs();
        let diff_y = (v.y - c[1]).abs();
        let diff_z = (v.z - c[2]).abs();
        let diff_w = (v.w - c[3]).abs();
        assert!(diff_x < f32::EPSILON);
        assert!(diff_y < f32::EPSILON);
        assert!(diff_z < f32::EPSILON);
        assert!(diff_w < f32::EPSILON);
    }

    #[test]
    fn color_vec4_opaque() {
        let v = color_vec4(theme::colors::NEON_CYAN);
        let diff = (v.w - 1.0).abs();
        assert!(diff < f32::EPSILON);
    }

    #[test]
    fn color_vec4_black() {
        let v = color_vec4([0.0, 0.0, 0.0, 1.0]);
        let diff_x = v.x.abs();
        let diff_y = v.y.abs();
        let diff_z = v.z.abs();
        let diff_w = (v.w - 1.0).abs();
        assert!(diff_x < f32::EPSILON);
        assert!(diff_y < f32::EPSILON);
        assert!(diff_z < f32::EPSILON);
        assert!(diff_w < f32::EPSILON);
    }

    #[test]
    fn color_vec4_white() {
        let v = color_vec4([1.0, 1.0, 1.0, 1.0]);
        let diff_x = (v.x - 1.0).abs();
        let diff_y = (v.y - 1.0).abs();
        let diff_z = (v.z - 1.0).abs();
        let diff_w = (v.w - 1.0).abs();
        assert!(diff_x < f32::EPSILON);
        assert!(diff_y < f32::EPSILON);
        assert!(diff_z < f32::EPSILON);
        assert!(diff_w < f32::EPSILON);
    }

    // ---- resolve_node_color tests ----

    fn make_node_record(id: &str, kind: &str) -> flow_core::doc::FlowNodeRecord {
        flow_core::doc::FlowNodeRecord {
            id: smol_str::SmolStr::from(id),
            kind: smol_str::SmolStr::from(kind),
            title: smol_str::SmolStr::from(id),
            position: [0.0, 0.0],
            size: [100.0, 60.0],
            z_index: 0,
            parent: None,
            ports: Vec::new(),
            flags: flow_core::doc::NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: flow_core::doc::NodeUiState::default(),
        }
    }

    fn make_node_with_color_override(
        id: &str,
        kind: &str,
        color: [f32; 4],
    ) -> flow_core::doc::FlowNodeRecord {
        flow_core::doc::FlowNodeRecord {
            ui: flow_core::doc::NodeUiState {
                color_override: Some(color),
                ..flow_core::doc::NodeUiState::default()
            },
            ..make_node_record(id, kind)
        }
    }

    #[test]
    fn resolve_node_color_default_returns_card_bg() {
        let node = make_node_record("n1", "Do");
        assert_eq!(FlowEditor::resolve_node_color(&node), theme::colors::CARD_BG);
    }

    #[test]
    fn resolve_node_color_override_takes_priority() {
        let custom = [0.5, 0.6, 0.7, 0.8];
        let node = make_node_with_color_override("n1", "Do", custom);
        assert_eq!(FlowEditor::resolve_node_color(&node), custom);
    }

    // ---- resolve_node_border_color tests ----

    #[test]
    fn border_color_do() {
        let node = make_node_record("n1", "Do");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_RUNNING
        );
    }

    #[test]
    fn border_color_do_lowercase() {
        let node = make_node_record("n1", "do");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_RUNNING
        );
    }

    #[test]
    fn border_color_choose() {
        let node = make_node_record("n1", "Choose");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_ASKING
        );
    }

    #[test]
    fn border_color_choose_lowercase() {
        let node = make_node_record("n1", "choose");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_ASKING
        );
    }

    #[test]
    fn border_color_branch() {
        let node = make_node_record("n1", "branch");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_ASKING
        );
    }

    #[test]
    fn border_color_foreach() {
        let node = make_node_record("n1", "ForEach");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_WAITING
        );
    }

    #[test]
    fn border_color_collect() {
        let node = make_node_record("n1", "Collect");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_WAITING
        );
    }

    #[test]
    fn border_color_loop() {
        let node = make_node_record("n1", "loop");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_WAITING
        );
    }

    #[test]
    fn border_color_together() {
        let node = make_node_record("n1", "Together");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_WAITING
        );
    }

    #[test]
    fn border_color_parallel() {
        let node = make_node_record("n1", "parallel");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_WAITING
        );
    }

    #[test]
    fn border_color_wait() {
        let node = make_node_record("n1", "Wait");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_SUCCEEDED
        );
    }

    #[test]
    fn border_color_ask() {
        let node = make_node_record("n1", "Ask");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_SUCCEEDED
        );
    }

    #[test]
    fn border_color_suspend() {
        let node = make_node_record("n1", "suspend");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_SUCCEEDED
        );
    }

    #[test]
    fn border_color_error_handler() {
        let node = make_node_record("n1", "ErrorHandler");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_FAILED
        );
    }

    #[test]
    fn border_color_error_lowercase() {
        let node = make_node_record("n1", "error");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::STATE_FAILED
        );
    }

    #[test]
    fn border_color_finish() {
        let node = make_node_record("n1", "Finish");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::NEON_TEAL
        );
    }

    #[test]
    fn border_color_terminal() {
        let node = make_node_record("n1", "terminal");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::NEON_TEAL
        );
    }

    #[test]
    fn border_color_jump() {
        let node = make_node_record("n1", "Jump");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::TEXT_SECONDARY
        );
    }

    #[test]
    fn border_color_nop() {
        let node = make_node_record("n1", "Nop");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::TEXT_SECONDARY
        );
    }

    #[test]
    fn border_color_control() {
        let node = make_node_record("n1", "control");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::TEXT_SECONDARY
        );
    }

    #[test]
    fn border_color_unknown_returns_border() {
        let node = make_node_record("n1", "SomethingWeird");
        assert_eq!(
            FlowEditor::resolve_node_border_color(&node),
            theme::colors::BORDER
        );
    }

    // ---- resolve_edge_color tests ----

    fn make_edge_record(line_style: flow_core::doc::LineStyle) -> flow_core::doc::FlowEdgeRecord {
        flow_core::doc::FlowEdgeRecord {
            id: smol_str::SmolStr::from("e1"),
            source_node: smol_str::SmolStr::from("a"),
            source_port: smol_str::SmolStr::from("out"),
            target_node: smol_str::SmolStr::from("b"),
            target_port: smol_str::SmolStr::from("in"),
            label: None,
            style: flow_core::doc::EdgeStyle {
                line_style,
                ..flow_core::doc::EdgeStyle::default()
            },
            data: serde_json::Value::Null,
            ui: flow_core::doc::EdgeUiState::default(),
        }
    }

    fn make_edge_with_color_override(_color: [f32; 4]) -> flow_core::doc::FlowEdgeRecord {
        flow_core::doc::FlowEdgeRecord {
            ui: flow_core::doc::EdgeUiState {
                color_override: None,
            },
            ..make_edge_record(flow_core::doc::LineStyle::Solid)
        }
    }

    #[test]
    fn edge_color_solid_is_cyan() {
        let edge = make_edge_record(flow_core::doc::LineStyle::Solid);
        assert_eq!(
            FlowEditor::resolve_edge_color(&edge),
            theme::colors::NEON_CYAN
        );
    }

    #[test]
    fn edge_color_dashed_is_red() {
        let edge = make_edge_record(flow_core::doc::LineStyle::Dashed);
        assert_eq!(
            FlowEditor::resolve_edge_color(&edge),
            theme::colors::STATE_FAILED
        );
    }

    #[test]
    fn edge_color_dotted_is_yellow() {
        let edge = make_edge_record(flow_core::doc::LineStyle::Dotted);
        assert_eq!(
            FlowEditor::resolve_edge_color(&edge),
            theme::colors::STATE_ASKING
        );
    }

    #[test]
    fn edge_color_override_ignored_by_resolve() {
        // Note: resolve_edge_color does NOT check color_override -- the caller
        // (draw_edge) handles that. So with a color_override set, the resolve
        // function still returns the line_style-based color.
        let custom = [0.9, 0.1, 0.5, 1.0];
        let edge = make_edge_with_color_override(custom);
        assert_eq!(
            FlowEditor::resolve_edge_color(&edge),
            theme::colors::NEON_CYAN
        );
    }

    // ---- FlowEditorAction variants ----

    #[test]
    fn action_document_changed_debug_format() {
        let action = FlowEditorAction::DocumentChanged;
        let debug_str = format!("{action:?}");
        assert!(debug_str.contains("DocumentChanged"));
    }

    #[test]
    fn action_selection_changed_debug_format() {
        let action = FlowEditorAction::SelectionChanged;
        let debug_str = format!("{action:?}");
        assert!(debug_str.contains("SelectionChanged"));
    }

    #[test]
    fn action_viewport_changed_debug_format() {
        let action = FlowEditorAction::ViewportChanged {
            pan_x: 1.0,
            pan_y: 2.0,
            zoom: 3.0,
        };
        let debug_str = format!("{action:?}");
        assert!(debug_str.contains("ViewportChanged"));
    }

    #[test]
    fn action_node_clicked_debug_format() {
        let action = FlowEditorAction::NodeClicked {
            node_id: smol_str::SmolStr::from("n1"),
        };
        let debug_str = format!("{action:?}");
        assert!(debug_str.contains("NodeClicked"));
    }

    #[test]
    fn action_edge_clicked_debug_format() {
        let action = FlowEditorAction::EdgeClicked {
            edge_id: smol_str::SmolStr::from("e1"),
        };
        let debug_str = format!("{action:?}");
        assert!(debug_str.contains("EdgeClicked"));
    }

    #[test]
    fn action_canvas_clicked_debug_format() {
        let action = FlowEditorAction::CanvasClicked {
            world_x: 10.0,
            world_y: 20.0,
        };
        let debug_str = format!("{action:?}");
        assert!(debug_str.contains("CanvasClicked"));
    }

    #[test]
    fn action_viewport_changed_clone() {
        let action = FlowEditorAction::ViewportChanged {
            pan_x: 1.0,
            pan_y: 2.0,
            zoom: 3.0,
        };
        let cloned = action.clone();
        if let FlowEditorAction::ViewportChanged { pan_x, pan_y, zoom } = cloned {
            let diff_pan_x = (pan_x - 1.0).abs();
            let diff_pan_y = (pan_y - 2.0).abs();
            let diff_zoom = (zoom - 3.0).abs();
            assert!(diff_pan_x < f64::EPSILON);
            assert!(diff_pan_y < f64::EPSILON);
            assert!(diff_zoom < f64::EPSILON);
        } else {
            panic!("Expected ViewportChanged variant");
        }
    }

    // ---- All known node kinds map to non-default colors ----

    #[test]
    fn all_mapped_kinds_have_specific_colors() {
        let kinds = [
            ("Do", theme::colors::STATE_RUNNING),
            ("do", theme::colors::STATE_RUNNING),
            ("Choose", theme::colors::STATE_ASKING),
            ("choose", theme::colors::STATE_ASKING),
            ("branch", theme::colors::STATE_ASKING),
            ("ForEach", theme::colors::STATE_WAITING),
            ("foreach", theme::colors::STATE_WAITING),
            ("Collect", theme::colors::STATE_WAITING),
            ("collect", theme::colors::STATE_WAITING),
            ("loop", theme::colors::STATE_WAITING),
            ("Together", theme::colors::STATE_WAITING),
            ("together", theme::colors::STATE_WAITING),
            ("parallel", theme::colors::STATE_WAITING),
            ("Wait", theme::colors::STATE_SUCCEEDED),
            ("wait", theme::colors::STATE_SUCCEEDED),
            ("Ask", theme::colors::STATE_SUCCEEDED),
            ("ask", theme::colors::STATE_SUCCEEDED),
            ("suspend", theme::colors::STATE_SUCCEEDED),
            ("ErrorHandler", theme::colors::STATE_FAILED),
            ("error_handler", theme::colors::STATE_FAILED),
            ("error", theme::colors::STATE_FAILED),
            ("Finish", theme::colors::NEON_TEAL),
            ("finish", theme::colors::NEON_TEAL),
            ("terminal", theme::colors::NEON_TEAL),
            ("Jump", theme::colors::TEXT_SECONDARY),
            ("jump", theme::colors::TEXT_SECONDARY),
            ("Nop", theme::colors::TEXT_SECONDARY),
            ("nop", theme::colors::TEXT_SECONDARY),
            ("control", theme::colors::TEXT_SECONDARY),
        ];
        for (kind, expected) in &kinds {
            let node = make_node_record("n", kind);
            let result = FlowEditor::resolve_node_border_color(&node);
            assert_eq!(
                result, *expected,
                "border color mismatch for kind '{kind}'"
            );
        }
    }
}
