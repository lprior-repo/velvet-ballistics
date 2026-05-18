#![forbid(unsafe_code)]

use crate::tokens::color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OverlayState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Waiting,
    Asking,
    Cancelled,
}

impl OverlayState {
    pub fn glow_color(self) -> [f32; 4] {
        match self {
            Self::Pending => color::pending(),
            Self::Running => color::running(),
            Self::Succeeded => color::success(),
            Self::Failed => color::failure(),
            Self::Skipped => color::text_tertiary(),
            Self::Waiting => color::active_cyan(),
            Self::Asking => color::warning(),
            Self::Cancelled => color::text_tertiary(),
        }
    }

    pub fn glow_radius(self) -> f32 {
        match self {
            Self::Pending => 2.0,
            Self::Running => 4.0,
            Self::Succeeded => 3.0,
            Self::Failed => 6.0,
            Self::Skipped => 2.0,
            Self::Waiting => 3.0,
            Self::Asking => 3.0,
            Self::Cancelled => 2.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodeBadge {
    ActionId(u16),
    RetryMax(u16),
    Timeout(u32),
    SecretSensitive,
    StrictDurable,
    RecentFailures(u32),
}

impl NodeBadge {
    pub fn label(&self) -> String {
        match self {
            Self::ActionId(id) => format!("A{}", id),
            Self::RetryMax(max) => format!("R{}", max),
            Self::Timeout(secs) => format!("T{}s", secs),
            Self::SecretSensitive => String::from("S"),
            Self::StrictDurable => String::from("D"),
            Self::RecentFailures(n) => format!("!{}", n),
        }
    }

    pub fn color(&self) -> [f32; 4] {
        match self {
            Self::ActionId(_) => [1.0, 0.42, 0.0, 1.0],
            Self::RetryMax(_) => [1.0, 0.9, 0.0, 1.0],
            Self::Timeout(_) => [1.0, 0.027, 0.227, 1.0],
            Self::SecretSensitive => [1.0, 0.0, 1.0, 1.0],
            Self::StrictDurable => [0.0, 0.898, 0.78, 1.0],
            Self::RecentFailures(_) => [1.0, 0.027, 0.227, 1.0],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeCardRenderInstr {
    pub step_idx: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub header_color: [f32; 4],
    pub body_color: [f32; 4],
    pub border_color: [f32; 4],
    pub text_color: [f32; 4],
    pub kind_label: String,
    pub badges: Vec<NodeBadge>,
    pub overlay_state: Option<OverlayState>,
    pub is_selected: bool,
    pub show_taint_overlay: bool,
}

impl NodeCardRenderInstr {
    pub fn focus_shadow_color() -> [f32; 4] {
        [0.122, 0.478, 0.961, 1.0]
    }

    pub fn failure_shadow_color() -> [f32; 4] {
        [0.898, 0.282, 0.302, 1.0]
    }

    pub fn taint_overlay_color() -> [f32; 4] {
        color::taint()
    }
}

pub struct GraphNode;

impl GraphNode {
    pub const NODE_WIDTH: f64 = 160.0;
    pub const NODE_HEIGHT: f64 = 48.0;
    pub const HEADER_HEIGHT: f64 = 24.0;

    pub fn card_dimensions() -> (f64, f64) {
        (Self::NODE_WIDTH, Self::NODE_HEIGHT)
    }

    pub fn header_dimensions() -> (f64, f64) {
        (Self::NODE_WIDTH, Self::HEADER_HEIGHT)
    }

    pub fn badge_size() -> f64 {
        16.0
    }
}
