#![forbid(unsafe_code)]

use crate::tokens::color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EdgeType {
    Normal,
    Branch,
    ErrorRoute,
    RetryRoute,
    Join,
    LoopBack,
}

impl EdgeType {
    pub fn color(self) -> [f32; 4] {
        match self {
            Self::Normal => [0.0, 0.6, 0.8, 1.0],
            Self::Branch => [0.694, 0.302, 1.0, 1.0],
            Self::ErrorRoute => [0.6, 0.1, 0.1, 1.0],
            Self::RetryRoute => [1.0, 0.9, 0.0, 1.0],
            Self::Join => [0.176, 0.42, 1.0, 1.0],
            Self::LoopBack => [0.0, 0.898, 0.78, 1.0],
        }
    }

    pub fn is_dashed(self) -> bool {
        matches!(self, Self::Branch | Self::ErrorRoute | Self::RetryRoute)
    }
}

#[derive(Debug, Clone)]
pub struct EdgeRenderInstr {
    pub source_step: usize,
    pub target_step: usize,
    pub start: [f64; 2],
    pub cp1: [f64; 2],
    pub cp2: [f64; 2],
    pub end: [f64; 2],
    pub edge_type: EdgeType,
    pub color: [f32; 4],
    pub width: f32,
    pub dashed: bool,
    pub label: Option<String>,
}

impl EdgeRenderInstr {
    pub fn from_edge_path(
        source_step: usize,
        target_step: usize,
        start: [f64; 2],
        cp1: [f64; 2],
        cp2: [f64; 2],
        end: [f64; 2],
        edge_type: EdgeType,
    ) -> Self {
        Self {
            source_step,
            target_step,
            start,
            cp1,
            cp2,
            end,
            edge_type,
            color: edge_type.color(),
            width: 2.0,
            dashed: edge_type.is_dashed(),
            label: None,
        }
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }
}

#[derive(Debug, Clone)]
pub struct PacketMarkerInstr {
    pub t: f64,
    pub color: [f32; 4],
    pub size: f32,
}

impl PacketMarkerInstr {
    pub fn new(t: f64) -> Self {
        Self {
            t: t.clamp(0.0, 1.0),
            color: color::active_cyan(),
            size: 6.0,
        }
    }
}

pub struct GraphEdge;

impl GraphEdge {
    pub const DEFAULT_WIDTH: f32 = 2.0;
    pub const HIGHLIGHT_WIDTH: f32 = 3.0;
    pub const PACKET_SIZE: f32 = 6.0;
}
