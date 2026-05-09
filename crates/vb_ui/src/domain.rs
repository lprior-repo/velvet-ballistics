#![forbid(unsafe_code)]
//! Domain types for the Mission Control UI.
//!
//! Wraps primitive values to eliminate primitive obsession and enforce
//! Farley constraints (each function ≤ 25 lines).

use makepad_widgets::{Rect, Vec4f};

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

/// X-axis offsets for the 5 navigation tabs in the header bar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TabOffsets(pub [f64; 5]);

impl TabOffsets {
    pub(crate) const fn new() -> Self {
        Self([0.0, 80.0, 160.0, 240.0, 330.0])
    }

    pub(crate) const TAB_WIDTH: f64 = 70.0;
    pub(crate) const TAB_HEIGHT: f64 = 28.0;
    pub(crate) const HEADER_HEIGHT: f64 = 45.0;
}

/// Layout constants for the transport (playback) bar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TransportLayout {
    pub(crate) transport_y_offset: f64,
    pub(crate) transport_height: f64,
    pub(crate) btn_width: f64,
    pub(crate) start_x_offset: f64,
}

impl TransportLayout {
    pub(crate) const CONTENT_Y_OFFSET: f64 = 73.0;
    pub(crate) const TRANSPORT_Y_OFFSET: f64 = 150.0;
    pub(crate) const TRANSPORT_HEIGHT: f64 = 50.0;
    pub(crate) const BTN_WIDTH: f64 = 30.0;
    pub(crate) const BTN_SPACING: f64 = 10.0;
    pub(crate) const START_X_OFFSET: f64 = 20.0;

    #[allow(elided_lifetimes_in_paths)]
    pub(crate) fn from_rect(rect: &Rect) -> Self {
        Self {
            transport_y_offset: rect.pos.y + Self::CONTENT_Y_OFFSET + Self::TRANSPORT_Y_OFFSET,
            transport_height: Self::TRANSPORT_HEIGHT,
            btn_width: Self::BTN_WIDTH,
            start_x_offset: Self::START_X_OFFSET,
        }
    }

    /// Returns button x positions: [|<, <, >, >|]
    pub(crate) fn button_positions(&self, _transport_start_x: f64) -> [f64; 4] {
        compute_button_positions()
    }
}

const fn compute_button_positions() -> [f64; 4] {
    let spacing = TransportLayout::BTN_WIDTH + TransportLayout::BTN_SPACING;
    [0.0, spacing, spacing * 2.0, spacing * 3.0]
}

/// Pre-computed color palette for the nav tabs (background + accent per tab).
#[derive(Debug, Clone)]
pub(crate) struct TabColors {
    pub(crate) bg: [f32; 3],
    pub(crate) accent: [f32; 3],
}

impl TabColors {
    pub(crate) fn for_tab(screen_index: usize, is_active: bool) -> Self {
        let (bg_r, bg_g, bg_b) = if is_active {
            (0.10_f32, 0.165_f32, 0.165_f32)
        } else {
            (0.102_f32, 0.102_f32, 0.180_f32)
        };

        let accent = match screen_index {
            0 => (0.0_f32, 0.96_f32, 1.0_f32),  // RunReplay - cyan
            1 => (0.22_f32, 1.0_f32, 0.08_f32), // Verification - green
            2 => (0.18_f32, 0.42_f32, 1.0_f32), // SystemOverview - blue
            3 => (0.69_f32, 0.30_f32, 1.0_f32), // WorkflowGraph - purple
            4 => (1.0_f32, 0.03_f32, 0.23_f32), // IncidentConsole - red
            _ => (0.5_f32, 0.5_f32, 0.5_f32),
        };

        Self {
            bg: [bg_r, bg_g, bg_b],
            accent: [accent.0, accent.1, accent.2],
        }
    }
}

/// Dark background color used for main content areas.
pub(crate) fn dark_bg_color() -> Vec4f {
    Vec4f {
        x: 0.039,
        y: 0.039,
        z: 0.071,
        w: 1.0,
    }
}

/// Header bar background color.
pub(crate) fn header_bg_color() -> Vec4f {
    Vec4f {
        x: 0.071,
        y: 0.078,
        z: 0.122,
        w: 1.0,
    }
}

/// Separator line color between header and content.
pub(crate) fn separator_color() -> Vec4f {
    Vec4f {
        x: 0.165,
        y: 0.165,
        z: 0.290,
        w: 1.0,
    }
}

/// Panel background color for content areas.
pub(crate) fn panel_bg_color() -> Vec4f {
    Vec4f {
        x: 0.086,
        y: 0.086,
        z: 0.165,
        w: 1.0,
    }
}
