#![forbid(unsafe_code)]
//! WorkflowCanvas - viewport state, selection, pan/zoom, focus-jump.

use std::collections::HashSet;

#[cfg(test)]
use std::collections::HashMap;

use crate::graph_builder::FlowDocument;
use crate::layout::{LayoutEdge, LayoutNode, LayoutResult};

use super::types::{
    EdgePath, ViewportRect, BEZIER_OFFSET, DEFAULT_ZOOM, MAX_ZOOM, MIN_ZOOM,
};

#[derive(Debug, Clone)]
pub struct WorkflowCanvas {
    document: FlowDocument,
    layout: LayoutResult,
    pan_x: f64,
    pan_y: f64,
    zoom: f64,
    selected: Option<usize>,
    node_ids: Vec<String>,
    collapsed_groups: HashSet<String>,
}

impl WorkflowCanvas {
    #[must_use]
    pub fn new(document: FlowDocument) -> Self {
        let entry_id = document
            .graph
            .entry_node
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");

        let (layout_nodes, node_ids) = Self::build_layout_nodes(&document);
        let layout_edges = Self::build_layout_edges(&document);

        let layout = crate::layout::compute_layout(&layout_nodes, &layout_edges, entry_id);

        Self {
            document,
            layout,
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: DEFAULT_ZOOM,
            selected: None,
            node_ids,
            collapsed_groups: HashSet::new(),
        }
    }

    #[must_use]
    pub fn document(&self) -> &FlowDocument {
        &self.document
    }

    #[must_use]
    pub fn layout(&self) -> &LayoutResult {
        &self.layout
    }

    #[must_use]
    pub fn pan(&self) -> (f64, f64) {
        (self.pan_x, self.pan_y)
    }

    #[must_use]
    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.selected
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

    #[must_use]
    pub fn zoom_percentage(&self) -> String {
        format!("{:.0}%", self.zoom * 100.0)
    }

    pub fn set_selected(&mut self, step: Option<usize>) {
        self.selected = step;
    }

    #[must_use]
    pub fn node_ids_slice(&self) -> &[String] {
        &self.node_ids
    }

    #[must_use]
    pub fn is_group_node(&self, node_id: &str) -> bool {
        if let Some(_node) = self.document.graph.nodes.get(node_id) {
            for group in self.document.graph.groups.values() {
                if group.children.iter().any(|c| c.as_str() == node_id) {
                    return true;
                }
            }
        }
        false
    }

    #[must_use]
    pub fn get_group_children_count(&self, node_id: &str) -> usize {
        if let Some(group) = self.document.graph.groups.get(node_id) {
            return group.children.len();
        }
        0
    }

    #[must_use]
    pub fn is_collapsed(&self, node_id: &str) -> bool {
        self.collapsed_groups.contains(node_id)
    }

    pub fn toggle_collapse(&mut self, node_id: &str) {
        if !self.collapsed_groups.remove(node_id) {
            self.collapsed_groups.insert(node_id.to_string());
        }
    }

    #[must_use]
    pub fn is_hidden_by_collapse(&self, node_id: &str) -> bool {
        for group_id in &self.collapsed_groups {
            if let Some(group) = self.document.graph.groups.get(group_id.as_str())
                && group.children.iter().any(|c| c.as_str() == node_id)
            {
                return true;
            }
        }
        false
    }

    #[must_use]
    pub fn collapsed_groups_set(&self) -> &HashSet<String> {
        &self.collapsed_groups
    }

    #[must_use]
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

    #[must_use]
    pub fn visible_nodes(&self, viewport: &ViewportRect) -> Vec<(usize, f64, f64, f64, f64)> {
        let mut result = Vec::new();
        for (idx, node_id) in self.node_ids.iter().enumerate() {
            if self.is_hidden_by_collapse(node_id.as_str()) {
                continue;
            }
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

            let nx = pos[0] - half_w;
            let ny = pos[1] - half_h;

            if viewport.intersects(nx, ny, node.size[0], node.size[1]) {
                result.push((idx, pos[0], pos[1], node.size[0], node.size[1]));
            }
        }
        result
    }

    pub fn focus_jump(&mut self, step_id: usize, screen_width: f64, screen_height: f64) -> bool {
        let node_id = match self.node_ids.get(step_id) {
            Some(id) => id.as_str(),
            None => return false,
        };

        let pos = match self.layout.positions.get(node_id) {
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

        self.pan_x = pos[0] - view_w / 2.0;
        self.pan_y = pos[1] - view_h / 2.0;
        true
    }

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

            let start = [src_pos[0] + src_size[0] / 2.0, src_pos[1]];
            let end = [tgt_pos[0] - tgt_size[0] / 2.0, tgt_pos[1]];

            let dx = (end[0] - start[0]).abs();
            let cp_offset = BEZIER_OFFSET.min(dx / 2.0).max(BEZIER_OFFSET / 2.0);

            let cp1 = [start[0] + cp_offset, start[1]];
            let cp2 = [end[0] - cp_offset, end[1]];

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

    #[must_use]
    pub fn node_count(&self) -> usize {
        self.node_ids.len()
    }

    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.document.graph.edges.len()
    }

    fn resolve_node(&self, node_id: &str) -> Option<(usize, [f64; 2], [f64; 2])> {
        let step_idx = self.node_ids.iter().position(|id| id.as_str() == node_id)?;

        let pos = self.layout.positions.get(node_id)?;
        let node = self.document.graph.nodes.get(node_id)?;
        Some((step_idx, *pos, node.size))
    }

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

    #[cfg(test)]
    pub fn test_positions(&self) -> HashMap<usize, [f64; 2]> {
        let mut map = HashMap::new();
        for (idx, node_id) in self.node_ids.iter().enumerate() {
            if let Some(&pos) = self.layout.positions.get(node_id.as_str()) {
                map.insert(idx, pos);
            }
        }
        map
    }
}
