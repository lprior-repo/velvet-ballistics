#![forbid(unsafe_code)]
//! Drawing helpers for the Mission Control UI.
//!
//! Contains low-level draw functions extracted from main.rs to satisfy
//! Farley constraints (≤ 25 lines per function).

use crate::domain::{
    TabColors, TabOffsets, dark_bg_color, header_bg_color, panel_bg_color, separator_color,
};
use makepad_widgets::*;
use vb_ui::app_state::{AppState, Screen};

const HEADER_HEIGHT: f64 = 44.0;

/// Draws the main dark background covering the entire widget.
#[allow(elided_lifetimes_in_paths)]
pub(crate) fn draw_background(draw_bg: &mut DrawColor, cx: &mut Cx2d, rect: Rect) {
    draw_bg.color = dark_bg_color();
    draw_bg.draw_abs(cx, rect);
}

/// Draws the header bar with title placeholder and separator line.
#[allow(elided_lifetimes_in_paths)]
pub(crate) fn draw_header_bar(draw_header: &mut DrawColor, cx: &mut Cx2d, rect: Rect) {
    let header_rect = header_geometry(rect);
    draw_header.color = header_bg_color();
    draw_header.draw_abs(cx, header_rect);
    draw_header_title(draw_header, cx, rect);
    draw_header_separator(draw_header, cx, rect);
}

fn header_geometry(rect: Rect) -> Rect {
    Rect {
        pos: rect.pos,
        size: DVec2 { x: rect.size.x, y: HEADER_HEIGHT },
    }
}

#[allow(elided_lifetimes_in_paths)]
fn draw_header_title(draw_header: &mut DrawColor, cx: &mut Cx2d, rect: Rect) {
    let title_rect = Rect {
        pos: DVec2 { x: rect.pos.x + 16.0, y: rect.pos.y + 8.0 },
        size: DVec2 { x: 40.0, y: 28.0 },
    };
    draw_header.color = Vec4f { x: 0.0, y: 0.96, z: 1.0, w: 1.0 };
    draw_header.draw_abs(cx, title_rect);
}

#[allow(elided_lifetimes_in_paths)]
fn draw_header_separator(draw_header: &mut DrawColor, cx: &mut Cx2d, rect: Rect) {
    let separator_rect = Rect {
        pos: DVec2 { x: rect.pos.x, y: rect.pos.y + HEADER_HEIGHT },
        size: DVec2 { x: rect.size.x, y: 1.0 },
    };
    draw_header.color = separator_color();
    draw_header.draw_abs(cx, separator_rect);
}

/// Draws the navigation tabs (5 tabs across the header).
#[allow(elided_lifetimes_in_paths)]
pub(crate) fn draw_nav_tabs(
    draw_nav: &mut DrawColor,
    cx: &mut Cx2d,
    rect: Rect,
    app_state: &AppState,
) {
    let offsets = TabOffsets::new();
    let y = rect.pos.y + TabOffsets::HEADER_HEIGHT;

    for (i, &x_offset) in offsets.0.iter().enumerate() {
        let is_active = match app_state.current_screen() {
            Screen::RunReplay => i == 0,
            Screen::Verification => i == 1,
            Screen::SystemOverview => i == 2,
            Screen::WorkflowGraph => i == 3,
            Screen::IncidentConsole => i == 4,
        };

        let colors = TabColors::for_tab(i, is_active);

        // Tab background
        let tab_rect = Rect {
            pos: DVec2 {
                x: rect.pos.x + x_offset,
                y,
            },
            size: DVec2 {
                x: TabOffsets::TAB_WIDTH,
                y: TabOffsets::TAB_HEIGHT,
            },
        };
        draw_nav.color = Vec4f {
            x: colors.bg[0],
            y: colors.bg[1],
            z: colors.bg[2],
            w: 1.0,
        };
        draw_nav.draw_abs(cx, tab_rect);

        // Tab accent (bottom border)
        let accent_rect = Rect {
            pos: DVec2 {
                x: rect.pos.x + x_offset,
                y: y + TabOffsets::TAB_HEIGHT - 3.0,
            },
            size: DVec2 {
                x: TabOffsets::TAB_WIDTH,
                y: 3.0,
            },
        };
        draw_nav.color = Vec4f {
            x: colors.accent[0],
            y: colors.accent[1],
            z: colors.accent[2],
            w: 1.0,
        };
        draw_nav.draw_abs(cx, accent_rect);
    }
}

/// Draws the main content area with a panel and accent border.
#[allow(elided_lifetimes_in_paths)]
pub(crate) fn draw_content(
    draw_bg: &mut DrawColor,
    cx: &mut Cx2d,
    rect: Rect,
    app_state: &AppState,
) {
    let content_y = rect.pos.y + 73.0;

    // Content background
    let content_rect = Rect {
        pos: DVec2 {
            x: rect.pos.x,
            y: content_y,
        },
        size: DVec2 {
            x: rect.size.x,
            y: rect.size.y - 73.0,
        },
    };
    draw_bg.color = dark_bg_color();
    draw_bg.draw_abs(cx, content_rect);

    // Panel background
    let panel_rect = Rect {
        pos: DVec2 {
            x: rect.pos.x + 20.0,
            y: content_y + 20.0,
        },
        size: DVec2 {
            x: rect.size.x - 40.0,
            y: 150.0,
        },
    };
    draw_bg.color = panel_bg_color();
    draw_bg.draw_abs(cx, panel_rect);

    // Accent border (left edge) colored by current screen
    let (r, g, b) = match app_state.current_screen() {
        Screen::RunReplay => (0.0, 0.96, 1.0),
        Screen::Verification => (0.22, 1.0, 0.08),
        Screen::SystemOverview => (0.18, 0.42, 1.0),
        Screen::WorkflowGraph => (0.69, 0.30, 1.0),
        Screen::IncidentConsole => (1.0, 0.03, 0.23),
    };
    let accent_rect = Rect {
        pos: DVec2 {
            x: rect.pos.x + 20.0,
            y: content_y + 20.0,
        },
        size: DVec2 { x: 4.0, y: 150.0 },
    };
    draw_bg.color = Vec4f {
        x: r,
        y: g,
        z: b,
        w: 1.0,
    };
    draw_bg.draw_abs(cx, accent_rect);
}
