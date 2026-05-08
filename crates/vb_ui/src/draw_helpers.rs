#![forbid(unsafe_code)]
//! Drawing helpers for the Mission Control UI.
//!
//! Contains low-level draw functions extracted from main.rs to satisfy
//! Farley constraints (≤ 25 lines per function).

use crate::domain::{
    TabColors, TabOffsets, dark_bg_color, header_bg_color, panel_bg_color, separator_color,
};
use makepad_widgets::*;
use vb_ui::app_state::{AppState, HealthLevel, Screen};

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

    // Per-screen placeholder content
    match app_state.current_screen() {
        Screen::SystemOverview => draw_system_overview_content(draw_bg, cx, &panel_rect, app_state),
        Screen::WorkflowGraph => draw_workflow_graph_content(draw_bg, cx, &panel_rect, app_state),
        Screen::RunReplay => draw_run_replay_content(draw_bg, cx, &panel_rect, app_state),
        Screen::Verification => draw_verification_content(draw_bg, cx, &panel_rect, app_state),
        Screen::IncidentConsole => draw_incident_content(draw_bg, cx, &panel_rect, app_state),
    }
}

#[allow(elided_lifetimes_in_paths)]
fn draw_system_overview_content(
    draw_bg: &mut DrawColor,
    cx: &mut Cx2d,
    panel: &Rect,
    app_state: &AppState,
) {
    let sys = &app_state.system;

    // Title bar placeholder (blue accent)
    let title_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 12.0 },
        size: DVec2 { x: 180.0, y: 18.0 },
    };
    draw_bg.color = Vec4f { x: 0.18, y: 0.42, z: 1.0, w: 1.0 };
    draw_bg.draw_abs(cx, title_rect);

    // Shard count metric bar
    let shard_bar = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 42.0 },
        size: DVec2 { x: f64::from(sys.shard_count).mul_add(8.0, 40.0).min(200.0), y: 12.0 },
    };
    draw_bg.color = Vec4f { x: 0.3, y: 0.5, z: 1.0, w: 1.0 };
    draw_bg.draw_abs(cx, shard_bar);

    // Active runs metric bar
    let runs_bar = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 62.0 },
        size: DVec2 { x: f64::from(sys.total_active_runs).mul_add(6.0, 40.0).min(200.0), y: 12.0 },
    };
    draw_bg.color = Vec4f { x: 0.0, y: 0.8, z: 1.0, w: 1.0 };
    draw_bg.draw_abs(cx, runs_bar);

    // Queue depth metric bar
    let queue_bar = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 82.0 },
        size: DVec2 { x: f64::from(sys.total_queue_depth).mul_add(4.0, 40.0).min(200.0), y: 12.0 },
    };
    draw_bg.color = Vec4f { x: 0.5, y: 0.7, z: 1.0, w: 1.0 };
    draw_bg.draw_abs(cx, queue_bar);

    // Health status indicator
    let health_color = match sys.overall_health {
        HealthLevel::Healthy => Vec4f { x: 0.22, y: 1.0, z: 0.08, w: 1.0 },
        HealthLevel::Degraded => Vec4f { x: 1.0, y: 0.9, z: 0.0, w: 1.0 },
        HealthLevel::Critical => Vec4f { x: 1.0, y: 0.03, z: 0.23, w: 1.0 },
    };
    let health_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 110.0 },
        size: DVec2 { x: 80.0, y: 20.0 },
    };
    draw_bg.color = health_color;
    draw_bg.draw_abs(cx, health_rect);
}

#[allow(elided_lifetimes_in_paths)]
fn draw_workflow_graph_content(
    draw_bg: &mut DrawColor,
    cx: &mut Cx2d,
    panel: &Rect,
    app_state: &AppState,
) {
    let wf = &app_state.workflow;

    // Title bar placeholder (purple accent)
    let title_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 12.0 },
        size: DVec2 { x: 180.0, y: 18.0 },
    };
    draw_bg.color = Vec4f { x: 0.69, y: 0.30, z: 1.0, w: 1.0 };
    draw_bg.draw_abs(cx, title_rect);

    // Workflow name placeholder
    let name_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 42.0 },
        size: DVec2 { x: 140.0, y: 14.0 },
    };
    draw_bg.color = Vec4f { x: 0.5, y: 0.4, z: 0.8, w: 1.0 };
    draw_bg.draw_abs(cx, name_rect);

    // Node count bar
    let node_bar = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 66.0 },
        size: DVec2 { x: f64::from(wf.node_count).mul_add(10.0, 30.0).min(200.0), y: 12.0 },
    };
    draw_bg.color = Vec4f { x: 0.8, y: 0.5, z: 1.0, w: 1.0 };
    draw_bg.draw_abs(cx, node_bar);

    // Graph canvas placeholder
    let canvas_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 240.0, y: panel.pos.y + 42.0 },
        size: DVec2 { x: 200.0, y: 90.0 },
    };
    draw_bg.color = Vec4f { x: 0.12, y: 0.10, z: 0.20, w: 1.0 };
    draw_bg.draw_abs(cx, canvas_rect);
}

#[allow(elided_lifetimes_in_paths)]
fn draw_run_replay_content(
    draw_bg: &mut DrawColor,
    cx: &mut Cx2d,
    panel: &Rect,
    app_state: &AppState,
) {
    let replay = &app_state.replay;

    // Title bar placeholder (cyan accent)
    let title_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 12.0 },
        size: DVec2 { x: 180.0, y: 18.0 },
    };
    draw_bg.color = Vec4f { x: 0.0, y: 0.96, z: 1.0, w: 1.0 };
    draw_bg.draw_abs(cx, title_rect);

    // Event count bar
    let event_bar = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 42.0 },
        size: DVec2 { x: f64::from(replay.total_events).mul_add(3.0, 40.0).min(200.0), y: 12.0 },
    };
    draw_bg.color = Vec4f { x: 0.0, y: 0.7, z: 0.9, w: 1.0 };
    draw_bg.draw_abs(cx, event_bar);

    // Playback position indicator
    let position_bar = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 62.0 },
        size: DVec2 { x: f64::from(replay.playback_position).mul_add(2.0, 40.0).min(200.0), y: 12.0 },
    };
    draw_bg.color = Vec4f { x: 0.2, y: 0.8, z: 1.0, w: 1.0 };
    draw_bg.draw_abs(cx, position_bar);

    // Speed indicator
    let speed_width = replay.playback_speed.mul_add(20.0, 40.0).min(120.0);
    let speed_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 82.0 },
        size: DVec2 { x: speed_width, y: 12.0 },
    };
    draw_bg.color = Vec4f { x: 0.4, y: 0.9, z: 1.0, w: 1.0 };
    draw_bg.draw_abs(cx, speed_rect);

    // Transport state indicator
    let transport_color = if replay.transport_state.is_playing() {
        Vec4f { x: 0.22, y: 1.0, z: 0.08, w: 1.0 }
    } else if replay.transport_state.is_paused() {
        Vec4f { x: 1.0, y: 0.9, z: 0.0, w: 1.0 }
    } else {
        Vec4f { x: 0.5, y: 0.5, z: 0.6, w: 1.0 }
    };
    let transport_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 110.0 },
        size: DVec2 { x: 60.0, y: 20.0 },
    };
    draw_bg.color = transport_color;
    draw_bg.draw_abs(cx, transport_rect);
}

#[allow(elided_lifetimes_in_paths)]
fn draw_verification_content(
    draw_bg: &mut DrawColor,
    cx: &mut Cx2d,
    panel: &Rect,
    app_state: &AppState,
) {
    let verify = &app_state.verification;

    // Title bar placeholder (green accent)
    let title_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 12.0 },
        size: DVec2 { x: 180.0, y: 18.0 },
    };
    draw_bg.color = Vec4f { x: 0.22, y: 1.0, z: 0.08, w: 1.0 };
    draw_bg.draw_abs(cx, title_rect);

    // Total checks bar
    let checks_bar = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 42.0 },
        size: DVec2 { x: f64::from(verify.total_checks).mul_add(15.0, 30.0).min(200.0), y: 12.0 },
    };
    draw_bg.color = Vec4f { x: 0.5, y: 0.8, z: 0.4, w: 1.0 };
    draw_bg.draw_abs(cx, checks_bar);

    // Pass count indicator
    let pass_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 62.0 },
        size: DVec2 { x: f64::from(verify.pass_count).mul_add(15.0, 20.0).min(120.0), y: 10.0 },
    };
    draw_bg.color = Vec4f { x: 0.22, y: 1.0, z: 0.08, w: 1.0 };
    draw_bg.draw_abs(cx, pass_rect);

    // Fail count indicator
    let fail_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 78.0 },
        size: DVec2 { x: f64::from(verify.fail_count).mul_add(15.0, 20.0).min(120.0), y: 10.0 },
    };
    draw_bg.color = Vec4f { x: 1.0, y: 0.03, z: 0.23, w: 1.0 };
    draw_bg.draw_abs(cx, fail_rect);

    // All-clean status indicator
    let status_color = if verify.all_clean {
        Vec4f { x: 0.22, y: 1.0, z: 0.08, w: 1.0 }
    } else if verify.fail_count > 0 {
        Vec4f { x: 1.0, y: 0.03, z: 0.23, w: 1.0 }
    } else {
        Vec4f { x: 1.0, y: 0.9, z: 0.0, w: 1.0 }
    };
    let status_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 100.0 },
        size: DVec2 { x: 80.0, y: 20.0 },
    };
    draw_bg.color = status_color;
    draw_bg.draw_abs(cx, status_rect);

    // Certificate panel placeholders (6 small rectangles)
    for i in 0..6 {
        let cert_rect = Rect {
            pos: DVec2 {
                x: panel.pos.x + 240.0 + f64::from(i) * 35.0,
                y: panel.pos.y + 42.0,
            },
            size: DVec2 { x: 28.0, y: 28.0 },
        };
        draw_bg.color = Vec4f { x: 0.1, y: 0.15, z: 0.1, w: 1.0 };
        draw_bg.draw_abs(cx, cert_rect);
    }
}

#[allow(elided_lifetimes_in_paths)]
fn draw_incident_content(
    draw_bg: &mut DrawColor,
    cx: &mut Cx2d,
    panel: &Rect,
    app_state: &AppState,
) {
    let inc = &app_state.incident;

    // Title bar placeholder (red accent)
    let title_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 12.0 },
        size: DVec2 { x: 180.0, y: 18.0 },
    };
    draw_bg.color = Vec4f { x: 1.0, y: 0.03, z: 0.23, w: 1.0 };
    draw_bg.draw_abs(cx, title_rect);

    // Active incidents bar
    let active_bar = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 42.0 },
        size: DVec2 { x: f64::from(inc.active_incidents).mul_add(20.0, 30.0).min(200.0), y: 14.0 },
    };
    draw_bg.color = Vec4f { x: 1.0, y: 0.2, z: 0.2, w: 1.0 };
    draw_bg.draw_abs(cx, active_bar);

    // Critical count indicator
    let crit_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 66.0 },
        size: DVec2 { x: f64::from(inc.critical_count).mul_add(20.0, 20.0).min(120.0), y: 10.0 },
    };
    draw_bg.color = Vec4f { x: 1.0, y: 0.03, z: 0.23, w: 1.0 };
    draw_bg.draw_abs(cx, crit_rect);

    // Warning count indicator
    let warn_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 82.0 },
        size: DVec2 { x: f64::from(inc.warning_count).mul_add(20.0, 20.0).min(120.0), y: 10.0 },
    };
    draw_bg.color = Vec4f { x: 1.0, y: 0.7, z: 0.0, w: 1.0 };
    draw_bg.draw_abs(cx, warn_rect);

    // Selected incident indicator
    let selected_color = if inc.selected_incident.is_some() {
        Vec4f { x: 1.0, y: 0.5, z: 0.0, w: 1.0 }
    } else {
        Vec4f { x: 0.3, y: 0.3, z: 0.4, w: 1.0 }
    };
    let selected_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 16.0, y: panel.pos.y + 104.0 },
        size: DVec2 { x: 80.0, y: 20.0 },
    };
    draw_bg.color = selected_color;
    draw_bg.draw_abs(cx, selected_rect);

    // Console placeholder
    let console_rect = Rect {
        pos: DVec2 { x: panel.pos.x + 240.0, y: panel.pos.y + 42.0 },
        size: DVec2 { x: 200.0, y: 90.0 },
    };
    draw_bg.color = Vec4f { x: 0.15, y: 0.08, z: 0.08, w: 1.0 };
    draw_bg.draw_abs(cx, console_rect);
}
