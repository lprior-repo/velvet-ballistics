#![forbid(unsafe_code)]
#![allow(clippy::arithmetic_side_effects)]
//! Domain types for the Mission Control UI.
//!
//! Wraps primitive values to eliminate primitive obsession and enforce
//! Farley constraints (each function ≤ 25 lines).

use makepad_widgets::{DVec2, Rect, Vec4f};

/// Number of consecutive clean IPC poll cycles before clearing an error state.
/// After 3 clean cycles, the error is considered resolved.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct IpcCleanCycles(pub u8);

impl IpcCleanCycles {
    pub(crate) const THRESHOLD: u8 = 3;

    pub(crate) fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }

    pub(crate) fn reset(&mut self) {
        self.0 = 0;
    }

    pub(crate) fn is_resolved(&self) -> bool {
        self.0 >= Self::THRESHOLD
    }
}

/// Shared shell metrics from the 11:51 Figma bundle.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ShellMetrics;

impl ShellMetrics {
    pub(crate) const OUTER_MARGIN: f64 = 32.0;
    pub(crate) const SIDEBAR_WIDTH: f64 = 246.0;
    pub(crate) const TOP_BAR_HEIGHT: f64 = 78.0;
    pub(crate) const CONTENT_GUTTER: f64 = 16.0;
    pub(crate) const HAIRLINE: f64 = 1.0;

    pub(crate) fn shell_rect(rect: Rect) -> Rect {
        Rect {
            pos: DVec2 {
                x: rect.pos.x + Self::OUTER_MARGIN,
                y: rect.pos.y + Self::OUTER_MARGIN,
            },
            size: DVec2 {
                x: rect.size.x - (Self::OUTER_MARGIN * 2.0),
                y: rect.size.y - (Self::OUTER_MARGIN * 2.0),
            },
        }
    }

    pub(crate) fn sidebar_rect(rect: Rect) -> Rect {
        let shell = Self::shell_rect(rect);
        Rect {
            pos: shell.pos,
            size: DVec2 {
                x: Self::SIDEBAR_WIDTH,
                y: shell.size.y,
            },
        }
    }

    pub(crate) fn top_bar_rect(rect: Rect) -> Rect {
        let shell = Self::shell_rect(rect);
        Rect {
            pos: DVec2 {
                x: shell.pos.x + Self::SIDEBAR_WIDTH + Self::CONTENT_GUTTER,
                y: shell.pos.y,
            },
            size: DVec2 {
                x: shell.size.x - Self::SIDEBAR_WIDTH - Self::CONTENT_GUTTER,
                y: Self::TOP_BAR_HEIGHT,
            },
        }
    }

    pub(crate) fn content_rect(rect: Rect) -> Rect {
        let top_bar = Self::top_bar_rect(rect);
        let shell = Self::shell_rect(rect);
        Rect {
            pos: DVec2 {
                x: top_bar.pos.x,
                y: top_bar.pos.y + top_bar.size.y + Self::CONTENT_GUTTER,
            },
            size: DVec2 {
                x: top_bar.size.x,
                y: shell.size.y - Self::TOP_BAR_HEIGHT - Self::CONTENT_GUTTER,
            },
        }
    }
}

/// Fixed left-sidebar navigation layout.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarLayout {
    pub(crate) x: f64,
    pub(crate) nav_y: f64,
    pub(crate) row_width: f64,
    pub(crate) row_height: f64,
    pub(crate) row_gap: f64,
}

impl SidebarLayout {
    pub(crate) const NAV_TOP_OFFSET: f64 = 118.0;
    pub(crate) const ROW_X_OFFSET: f64 = 16.0;
    pub(crate) const ROW_WIDTH: f64 = 214.0;
    pub(crate) const ROW_HEIGHT: f64 = 34.0;
    pub(crate) const ROW_GAP: f64 = 8.0;

    pub(crate) fn from_rect(rect: Rect) -> Self {
        let sidebar = ShellMetrics::sidebar_rect(rect);
        Self {
            x: sidebar.pos.x + Self::ROW_X_OFFSET,
            nav_y: sidebar.pos.y + Self::NAV_TOP_OFFSET,
            row_width: Self::ROW_WIDTH,
            row_height: Self::ROW_HEIGHT,
            row_gap: Self::ROW_GAP,
        }
    }

    pub(crate) fn row_rect(&self, row: u32) -> Rect {
        let offset = f64::from(row) * (self.row_height + self.row_gap);
        Rect {
            pos: DVec2 {
                x: self.x,
                y: self.nav_y + offset,
            },
            size: DVec2 {
                x: self.row_width,
                y: self.row_height,
            },
        }
    }
}

/// Layout constants for the transport (playback) bar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TransportLayout {
    pub(crate) transport_x: f64,
    pub(crate) transport_y: f64,
    pub(crate) transport_height: f64,
    pub(crate) btn_width: f64,
}

impl TransportLayout {
    pub(crate) const TRANSPORT_Y_OFFSET: f64 = 420.0;
    pub(crate) const TRANSPORT_HEIGHT: f64 = 40.0;
    pub(crate) const BTN_WIDTH: f64 = 74.0;
    pub(crate) const BTN_SPACING: f64 = 10.0;
    pub(crate) const START_X_OFFSET: f64 = 32.0;

    #[allow(elided_lifetimes_in_paths)]
    pub(crate) fn from_rect(rect: &Rect) -> Self {
        let content = ShellMetrics::content_rect(*rect);
        Self {
            transport_x: content.pos.x + Self::START_X_OFFSET,
            transport_y: content.pos.y + Self::TRANSPORT_Y_OFFSET,
            transport_height: Self::TRANSPORT_HEIGHT,
            btn_width: Self::BTN_WIDTH,
        }
    }

    /// Returns button x positions: [|<, <, play/pause, >, >|]
    pub(crate) fn button_positions(&self) -> [f64; 5] {
        compute_button_positions()
    }
}

const fn compute_button_positions() -> [f64; 5] {
    let spacing = TransportLayout::BTN_WIDTH + TransportLayout::BTN_SPACING;
    [0.0, spacing, spacing * 2.0, spacing * 3.0, spacing * 4.0]
}

const fn rgba(x: f32, y: f32, z: f32, w: f32) -> Vec4f {
    Vec4f { x, y, z, w }
}

pub(crate) fn app_bg_color() -> Vec4f {
    rgba(0.957, 0.965, 0.976, 1.0)
}

pub(crate) fn surface_color() -> Vec4f {
    rgba(1.0, 1.0, 1.0, 1.0)
}

pub(crate) fn panel_color() -> Vec4f {
    rgba(0.984, 0.988, 0.996, 1.0)
}

pub(crate) fn border_color() -> Vec4f {
    rgba(0.898, 0.918, 0.945, 1.0)
}

pub(crate) fn primary_text_color() -> Vec4f {
    rgba(0.059, 0.09, 0.165, 1.0)
}

pub(crate) fn secondary_text_color() -> Vec4f {
    rgba(0.263, 0.322, 0.4, 1.0)
}

pub(crate) fn muted_text_color() -> Vec4f {
    rgba(0.541, 0.596, 0.667, 1.0)
}

pub(crate) fn success_color() -> Vec4f {
    rgba(0.086, 0.651, 0.416, 1.0)
}

pub(crate) fn warning_color() -> Vec4f {
    rgba(0.961, 0.62, 0.043, 1.0)
}

pub(crate) fn failure_color() -> Vec4f {
    rgba(0.898, 0.282, 0.302, 1.0)
}

pub(crate) fn primary_blue_color() -> Vec4f {
    rgba(0.145, 0.388, 0.922, 1.0)
}

pub(crate) fn accent_from_rgba(color: [f32; 4]) -> Vec4f {
    let [x, y, z, w] = color;
    rgba(x, y, z, w)
}
