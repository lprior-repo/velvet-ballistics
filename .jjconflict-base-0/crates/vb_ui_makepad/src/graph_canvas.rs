#![forbid(unsafe_code)]

use crate::graph_edge::{EdgeRenderInstr, EdgeType, PacketMarkerInstr};
use crate::graph_node::{NodeBadge, NodeCardRenderInstr, OverlayState};
use crate::tokens::color;

#[derive(Debug, Clone)]
pub struct ViewportRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ViewportRect {
    pub fn intersects(&self, nx: f64, ny: f64, nw: f64, nh: f64) -> bool {
        let self_right = self.x + self.width;
        let self_bottom = self.y + self.height;
        let other_right = nx + nw;
        let other_bottom = ny + nh;

        !(self_right <= nx || other_right <= self.x || self_bottom <= ny || other_bottom <= self.y)
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

const MIN_ZOOM: f64 = 0.1;
const MAX_ZOOM: f64 = 5.0;
const DEFAULT_ZOOM: f64 = 1.0;

#[derive(Debug, Clone)]
pub struct GraphCanvas {
    node_count: usize,
    edge_count: usize,
    pan_x: f64,
    pan_y: f64,
    zoom: f64,
    selected: Option<usize>,
    node_positions: Vec<(f64, f64)>,
    edge_paths: Vec<EdgePath>,
    node_overlays: Vec<Option<OverlayState>>,
    taint_overlay_active: bool,
}

impl GraphCanvas {
    pub fn new(
        node_count: usize,
        node_positions: Vec<(f64, f64)>,
        edge_paths: Vec<EdgePath>,
    ) -> Self {
        Self {
            node_count,
            edge_count: edge_paths.len(),
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: DEFAULT_ZOOM,
            selected: None,
            node_positions,
            edge_paths,
            node_overlays: vec![None; node_count],
            taint_overlay_active: false,
        }
    }

    pub fn viewport_rect(&self, screen_width: f64, screen_height: f64) -> ViewportRect {
        let inv_zoom = if self.zoom > 0.0 {
            1.0 / self.zoom
        } else {
            1.0
        };
        ViewportRect {
            x: self.pan_x,
            y: self.pan_y,
            width: screen_width * inv_zoom,
            height: screen_height * inv_zoom,
        }
    }

    pub fn visible_nodes(
        &self,
        viewport: &ViewportRect,
        node_size: (f64, f64),
    ) -> Vec<(usize, f64, f64, f64, f64)> {
        let mut result = Vec::new();
        let (node_w, node_h) = node_size;
        let half_w = node_w / 2.0;
        let half_h = node_h / 2.0;

        for (idx, &(x, y)) in self.node_positions.iter().enumerate() {
            let nx = x - half_w;
            let ny = y - half_h;

            if viewport.intersects(nx, ny, node_w, node_h) {
                result.push((idx, x, y, node_w, node_h));
            }
        }
        result
    }

    pub fn compute_edge_paths(&self) -> Vec<EdgePath> {
        self.edge_paths.clone()
    }

    pub fn set_pan(&mut self, x: f64, y: f64) {
        self.pan_x = x;
        self.pan_y = y;
    }

    pub fn set_zoom(&mut self, zoom: f64) {
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    pub fn zoom_in(&mut self, factor: f64) {
        let new_zoom = self.zoom * factor;
        self.set_zoom(new_zoom);
    }

    pub fn zoom_out(&mut self, factor: f64) {
        let new_zoom = self.zoom / factor;
        self.set_zoom(new_zoom);
    }

    pub fn zoom_reset(&mut self) {
        self.zoom = DEFAULT_ZOOM;
    }

    pub fn zoom_percentage(&self) -> String {
        format!("{:.0}%", self.zoom * 100.0)
    }

    pub fn set_selected(&mut self, step: Option<usize>) {
        self.selected = step;
    }

    pub fn focus_jump(&mut self, step_id: usize, screen_width: f64, screen_height: f64) -> bool {
        let pos = match self.node_positions.get(step_id) {
            Some(&p) => p,
            None => return false,
        };

        let inv_zoom = if self.zoom > 0.0 {
            1.0 / self.zoom
        } else {
            1.0
        };
        let view_w = screen_width * inv_zoom;
        let view_h = screen_height * inv_zoom;

        self.pan_x = pos.0 - view_w / 2.0;
        self.pan_y = pos.1 - view_h / 2.0;
        true
    }

    pub fn node_layout_position(&self, step_idx: usize) -> Option<(f64, f64)> {
        self.node_positions.get(step_idx).copied()
    }

    pub fn render_node_card(&self, step_idx: usize) -> Option<NodeCardRenderInstr> {
        let &(x, y) = self.node_positions.get(step_idx)?;
        let overlay = self.node_overlays.get(step_idx).copied().flatten();
        let is_selected = self.selected == Some(step_idx);

        let body_color = color::surface();
        let text_color = color::text_primary();
        let border_color = if is_selected {
            NodeCardRenderInstr::focus_shadow_color()
        } else if let Some(Some(OverlayState::Failed)) = self.node_overlays.get(step_idx) {
            NodeCardRenderInstr::failure_shadow_color()
        } else {
            color::line_hair()
        };

        Some(NodeCardRenderInstr {
            step_idx,
            x,
            y,
            width: 160.0,
            height: 48.0,
            header_color: color::shell(),
            body_color,
            border_color,
            text_color,
            kind_label: String::new(),
            badges: Vec::new(),
            overlay_state: overlay,
            is_selected,
            show_taint_overlay: self.taint_overlay_active,
        })
    }

    pub fn node_status_dot_color(&self, step_idx: usize) -> Option<[f32; 4]> {
        self.node_overlays
            .get(step_idx)
            .copied()
            .flatten()
            .map(|s| s.glow_color())
    }

    pub fn node_badges(&self, _step_idx: usize) -> Vec<NodeBadge> {
        Vec::new()
    }

    pub fn render_edge(&self, edge_id: &str) -> Option<EdgeRenderInstr> {
        let idx = edge_id.parse::<usize>().ok()?;
        let path = self.edge_paths.get(idx)?;

        Some(EdgeRenderInstr::from_edge_path(
            path.source_step,
            path.target_step,
            path.start,
            path.cp1,
            path.cp2,
            path.end,
            EdgeType::Normal,
        ))
    }

    pub fn edge_packet_markers(&self, _edge_id: &str) -> Vec<PacketMarkerInstr> {
        Vec::new()
    }

    pub fn packet_dot_position(&self, _edge_id: &str, _t: f64) -> Option<[f64; 2]> {
        None
    }

    pub fn animate_packet_dots(&mut self, _delta_ms: f64) {}

    pub fn set_node_overlay(&mut self, step_idx: usize, state: Option<OverlayState>) {
        if let Some(slot) = self.node_overlays.get_mut(step_idx) {
            *slot = state;
        }
    }

    pub fn set_taint_overlay(&mut self, active: bool) {
        self.taint_overlay_active = active;
    }

    pub fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn edge_count(&self) -> usize {
        self.edge_count
    }

    pub fn pan(&self) -> (f64, f64) {
        (self.pan_x, self.pan_y)
    }

    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }
}
