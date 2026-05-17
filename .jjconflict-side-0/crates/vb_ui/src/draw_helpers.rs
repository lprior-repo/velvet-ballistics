#![forbid(unsafe_code)]
#![allow(clippy::arithmetic_side_effects)]
//! White Makepad shell renderer for the canonical eight-screen UI.

use crate::domain::{
    ShellMetrics, SidebarLayout, TransportLayout, accent_from_rgba, app_bg_color, border_color,
    failure_color, muted_text_color, panel_color, primary_blue_color, primary_text_color,
    secondary_text_color, success_color, surface_color, warning_color,
};
use makepad_widgets::*;
use vb_ui::app_state::{AppState, Screen};
use vb_ui::workflow::WorkflowCanvas;

const CARD_GAP: f64 = 16.0;
const CARD_HEIGHT: f64 = 148.0;
const HERO_HEIGHT: f64 = 122.0;

#[derive(Clone, Copy)]
enum Tone {
    Neutral,
    Blue,
    Green,
    Purple,
    Warning,
    Failure,
}

#[derive(Clone, Copy)]
struct PanelSpec {
    title: &'static str,
    metric: &'static str,
    detail: &'static str,
    tone: Tone,
}

#[derive(Clone, Copy)]
struct ScreenSpec {
    hero_title: &'static str,
    hero_detail: &'static str,
    left_title: &'static str,
    right_title: &'static str,
    panels: &'static [PanelSpec],
}

const OVERVIEW_PANELS: &[PanelSpec] = &[
    PanelSpec {
        title: "Active runs",
        metric: "23",
        detail: "running across shards",
        tone: Tone::Blue,
    },
    PanelSpec {
        title: "Healthy actions",
        metric: "1,245",
        detail: "99.2k/sec",
        tone: Tone::Green,
    },
    PanelSpec {
        title: "Verification pass rate",
        metric: "99.8%",
        detail: "last 24h",
        tone: Tone::Green,
    },
    PanelSpec {
        title: "Queue depth",
        metric: "178",
        detail: "medium pressure",
        tone: Tone::Warning,
    },
];

const WORKFLOW_PANELS: &[PanelSpec] = &[
    PanelSpec {
        title: "Selected workflow",
        metric: "issue_triage",
        detail: "draft projected as numeric IR",
        tone: Tone::Purple,
    },
    PanelSpec {
        title: "Selected step",
        metric: "create_issue",
        detail: "github.issue.create",
        tone: Tone::Blue,
    },
    PanelSpec {
        title: "Retry policy",
        metric: "max 2",
        detail: "backoff 1s",
        tone: Tone::Neutral,
    },
    PanelSpec {
        title: "Taint flow",
        metric: "Clean",
        detail: "input/output certified",
        tone: Tone::Green,
    },
];

const DETAILS_PANELS: &[PanelSpec] = &[
    PanelSpec {
        title: "Run",
        metric: "run_7273",
        detail: "workflow issue_triage",
        tone: Tone::Blue,
    },
    PanelSpec {
        title: "Attempt",
        metric: "1 of 2",
        detail: "elapsed 00:00:03",
        tone: Tone::Warning,
    },
    PanelSpec {
        title: "Shard",
        metric: "1",
        detail: "idempotency key issue_triage:8421",
        tone: Tone::Neutral,
    },
    PanelSpec {
        title: "Event stream",
        metric: "seq 12",
        detail: "create_issue started",
        tone: Tone::Blue,
    },
];

const VERIFY_PANELS: &[PanelSpec] = &[
    PanelSpec {
        title: "Structure valid",
        metric: "PASS",
        detail: "CFG accepted",
        tone: Tone::Green,
    },
    PanelSpec {
        title: "Boundedness",
        metric: "PASS",
        detail: "finite paths",
        tone: Tone::Green,
    },
    PanelSpec {
        title: "Resources",
        metric: "PASS",
        detail: "within limits",
        tone: Tone::Green,
    },
    PanelSpec {
        title: "Capabilities",
        metric: "PASS",
        detail: "least privilege",
        tone: Tone::Green,
    },
];

const REPLAY_PANELS: &[PanelSpec] = &[
    PanelSpec {
        title: "Selected event",
        metric: "seq 12",
        detail: "10:42:21.132",
        tone: Tone::Blue,
    },
    PanelSpec {
        title: "Recovery strategy",
        metric: "Retry",
        detail: "same idempotency key",
        tone: Tone::Green,
    },
    PanelSpec {
        title: "Slot diff",
        metric: "3 fields",
        detail: "title/repo/attempt",
        tone: Tone::Purple,
    },
    PanelSpec {
        title: "Safety",
        metric: "Safe",
        detail: "action scheduled durable",
        tone: Tone::Green,
    },
];

const INCIDENT_PANELS: &[PanelSpec] = &[
    PanelSpec {
        title: "Failure",
        metric: "ACTION_TIMEOUT",
        detail: "create_issue",
        tone: Tone::Failure,
    },
    PanelSpec {
        title: "Retry safety",
        metric: "YES",
        detail: "same key required",
        tone: Tone::Green,
    },
    PanelSpec {
        title: "Evidence",
        metric: "2 events",
        detail: "started + timeout",
        tone: Tone::Warning,
    },
    PanelSpec {
        title: "Runbook",
        metric: "RB-012",
        detail: "GitHub Actions",
        tone: Tone::Neutral,
    },
];

const ACTION_PANELS: &[PanelSpec] = &[
    PanelSpec {
        title: "Selected action",
        metric: "ActionId(7)",
        detail: "github.issue.create",
        tone: Tone::Warning,
    },
    PanelSpec {
        title: "Policy",
        metric: "Idempotent",
        detail: "external write",
        tone: Tone::Green,
    },
    PanelSpec {
        title: "Capabilities",
        metric: "3",
        detail: "github, secrets, journal",
        tone: Tone::Purple,
    },
    PanelSpec {
        title: "Schema",
        metric: "4 in / 2 out",
        detail: "postcard binary",
        tone: Tone::Blue,
    },
];

const STORAGE_PANELS: &[PanelSpec] = &[
    PanelSpec {
        title: "Storage doctor",
        metric: "PASS",
        detail: "Fjall lock held",
        tone: Tone::Green,
    },
    PanelSpec {
        title: "Journal events",
        metric: "12,482",
        detail: "monotonic SeqNo",
        tone: Tone::Blue,
    },
    PanelSpec {
        title: "Blob bytes",
        metric: "2.1 GiB",
        detail: "summarized for AI",
        tone: Tone::Purple,
    },
    PanelSpec {
        title: "Trim threshold",
        metric: "WARN",
        detail: "trim recommended soon",
        tone: Tone::Warning,
    },
];

#[allow(elided_lifetimes_in_paths)]
pub(crate) fn draw_background(draw_bg: &mut DrawColor, cx: &mut Cx2d, rect: Rect) {
    draw_rect(draw_bg, cx, rect, app_bg_color());
    draw_rect(draw_bg, cx, ShellMetrics::shell_rect(rect), border_color());
    draw_inset_rect(
        draw_bg,
        cx,
        ShellMetrics::shell_rect(rect),
        surface_color(),
        ShellMetrics::HAIRLINE,
    );
}

#[allow(elided_lifetimes_in_paths)]
pub(crate) fn draw_header_bar(draw_header: &mut DrawColor, cx: &mut Cx2d, rect: Rect) {
    draw_card(
        draw_header,
        cx,
        ShellMetrics::top_bar_rect(rect),
        surface_color(),
    );
}

#[allow(elided_lifetimes_in_paths)]
pub(crate) fn draw_nav_tabs(
    draw_nav: &mut DrawColor,
    cx: &mut Cx2d,
    rect: Rect,
    app_state: &AppState,
) {
    draw_card(
        draw_nav,
        cx,
        ShellMetrics::sidebar_rect(rect),
        surface_color(),
    );
    draw_nav_row(draw_nav, cx, rect, app_state, Screen::ExecutionOverview, 0);
    draw_nav_row(
        draw_nav,
        cx,
        rect,
        app_state,
        Screen::WorkflowGraphAuthoring,
        1,
    );
    draw_nav_row(
        draw_nav,
        cx,
        rect,
        app_state,
        Screen::ExecutionDetailsGraph,
        2,
    );
    draw_nav_row(
        draw_nav,
        cx,
        rect,
        app_state,
        Screen::VerificationCertificate,
        3,
    );
    draw_nav_row(draw_nav, cx, rect, app_state, Screen::ReplayTheater, 4);
    draw_nav_row(
        draw_nav,
        cx,
        rect,
        app_state,
        Screen::IncidentFailureConsole,
        5,
    );
    draw_nav_row(draw_nav, cx, rect, app_state, Screen::ActionRegistry, 6);
    draw_nav_row(
        draw_nav,
        cx,
        rect,
        app_state,
        Screen::StorageDoctorAiContext,
        7,
    );
    draw_nav_row(
        draw_nav,
        cx,
        rect,
        app_state,
        Screen::StorageDoctorAiContext,
        8,
    );
}

#[allow(elided_lifetimes_in_paths)]
pub(crate) fn draw_content(
    draw_bg: &mut DrawColor,
    _draw_vector: &mut DrawVector,
    draw_text: &mut DrawText,
    cx: &mut Cx2d,
    rect: Rect,
    app_state: &AppState,
    _workflow_canvas: &Option<WorkflowCanvas>,
) {
    draw_sidebar_text(draw_text, cx, rect);
    draw_top_bar_text(draw_bg, draw_text, cx, rect, app_state);
    draw_screen(
        draw_bg,
        draw_text,
        cx,
        ShellMetrics::content_rect(rect),
        app_state,
    );
    if app_state.show_shortcuts {
        draw_shortcuts(draw_bg, draw_text, cx, rect);
    }
}

#[allow(elided_lifetimes_in_paths)]
fn draw_screen(
    draw_bg: &mut DrawColor,
    draw_text: &mut DrawText,
    cx: &mut Cx2d,
    content: Rect,
    app_state: &AppState,
) {
    let spec = screen_spec(app_state.current_screen());
    draw_hero(draw_bg, draw_text, cx, content, app_state, spec);
    draw_panel_grid(draw_bg, draw_text, cx, content, spec.panels);
    draw_wide_panels(draw_bg, draw_text, cx, content, spec);
    if app_state.current_screen() == Screen::ReplayTheater {
        draw_transport(draw_bg, draw_text, cx, content);
    }
}

fn screen_spec(screen: Screen) -> ScreenSpec {
    match screen {
        Screen::ExecutionOverview => ScreenSpec {
            hero_title: "System overview",
            hero_detail: "Active runs, queue pressure, shard flow, and event ticker stay visible without red overload.",
            left_title: "Executions",
            right_title: "Shard flow map",
            panels: OVERVIEW_PANELS,
        },
        Screen::WorkflowGraphAuthoring => ScreenSpec {
            hero_title: "Verified workflow canvas",
            hero_detail: "State palette, retained node coordinates, semantic edge packets, and fixed inspector width.",
            left_title: "Canvas",
            right_title: "Step inspector",
            panels: WORKFLOW_PANELS,
        },
        Screen::ExecutionDetailsGraph => ScreenSpec {
            hero_title: "Run inspection",
            hero_detail: "Runtime graph, step details, and event stream remain tied to the selected run evidence.",
            left_title: "Runtime graph",
            right_title: "Event stream",
            panels: DETAILS_PANELS,
        },
        Screen::VerificationCertificate => ScreenSpec {
            hero_title: "Verification passed",
            hero_detail: "All admission gates passed with deterministic certificate and digest evidence.",
            left_title: "Certificate gates",
            right_title: "Accepted artifact",
            panels: VERIFY_PANELS,
        },
        Screen::ReplayTheater => ScreenSpec {
            hero_title: "Journal replay",
            hero_detail: "Scrubber-driven graph state, selected event marker, slot diff, and safe recovery decision.",
            left_title: "Replay graph",
            right_title: "Selected event",
            panels: REPLAY_PANELS,
        },
        Screen::IncidentFailureConsole => ScreenSpec {
            hero_title: "ACTION_TIMEOUT at create_issue",
            hero_detail: "Failure path is restrained red; evidence, retry safety, and repair hints take priority.",
            left_title: "Failure path",
            right_title: "Evidence chain",
            panels: INCIDENT_PANELS,
        },
        Screen::ActionRegistry => ScreenSpec {
            hero_title: "Action contracts",
            hero_detail: "Numeric ActionId policy, side effects, capabilities, retry safety, and binary schemas.",
            left_title: "Action registry",
            right_title: "Contract inspector",
            panels: ACTION_PANELS,
        },
        Screen::StorageDoctorAiContext => ScreenSpec {
            hero_title: "Storage doctor passed",
            hero_detail: "Fjall keyspaces, postcard envelopes, journal health, and AI-safe context packet.",
            left_title: "Journal doctor",
            right_title: "AI context packet",
            panels: STORAGE_PANELS,
        },
    }
}

#[allow(elided_lifetimes_in_paths)]
fn draw_hero(
    draw_bg: &mut DrawColor,
    draw_text: &mut DrawText,
    cx: &mut Cx2d,
    content: Rect,
    app_state: &AppState,
    spec: ScreenSpec,
) {
    let hero = Rect {
        pos: content.pos,
        size: DVec2 {
            x: content.size.x,
            y: HERO_HEIGHT,
        },
    };
    draw_card(draw_bg, cx, hero, surface_color());
    draw_accent(draw_bg, cx, hero, app_state.screen_nav_color());
    draw_label(
        draw_text,
        cx,
        hero.pos.x + 24.0,
        hero.pos.y + 22.0,
        spec.hero_title,
        20.0,
        primary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        hero.pos.x + 24.0,
        hero.pos.y + 56.0,
        spec.hero_detail,
        12.0,
        secondary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        hero.pos.x + 24.0,
        hero.pos.y + 84.0,
        "Figma-ready white Makepad shell",
        11.0,
        muted_text_color(),
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_panel_grid(
    draw_bg: &mut DrawColor,
    draw_text: &mut DrawText,
    cx: &mut Cx2d,
    content: Rect,
    panels: &[PanelSpec],
) {
    panels.iter().enumerate().for_each(|(index, panel)| {
        let col = usize_to_f64(index % 4);
        let card_width = (content.size.x - (CARD_GAP * 3.0)) / 4.0;
        let x = content.pos.x + (card_width + CARD_GAP) * col;
        let y = content.pos.y + HERO_HEIGHT + CARD_GAP;
        draw_metric_card(draw_bg, draw_text, cx, x, y, card_width, panel);
    });
}

#[allow(elided_lifetimes_in_paths)]
fn draw_wide_panels(
    draw_bg: &mut DrawColor,
    draw_text: &mut DrawText,
    cx: &mut Cx2d,
    content: Rect,
    spec: ScreenSpec,
) {
    let y = content.pos.y + HERO_HEIGHT + CARD_GAP + CARD_HEIGHT + CARD_GAP;
    let left_width = (content.size.x * 0.62) - (CARD_GAP * 0.5);
    let right_width = content.size.x - left_width - CARD_GAP;
    let height = content.size.y - HERO_HEIGHT - CARD_HEIGHT - (CARD_GAP * 2.0);
    let left = Rect {
        pos: DVec2 {
            x: content.pos.x,
            y,
        },
        size: DVec2 {
            x: left_width,
            y: height,
        },
    };
    let right = Rect {
        pos: DVec2 {
            x: content.pos.x + left_width + CARD_GAP,
            y,
        },
        size: DVec2 {
            x: right_width,
            y: height,
        },
    };
    draw_card(draw_bg, cx, left, surface_color());
    draw_card(draw_bg, cx, right, surface_color());
    draw_label(
        draw_text,
        cx,
        left.pos.x + 20.0,
        left.pos.y + 20.0,
        spec.left_title,
        16.0,
        primary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        right.pos.x + 20.0,
        right.pos.y + 20.0,
        spec.right_title,
        16.0,
        primary_text_color(),
    );
    draw_panel_lines(draw_text, cx, left);
    draw_inspector_lines(draw_text, cx, right);
}

#[allow(elided_lifetimes_in_paths)]
fn draw_metric_card(
    draw_bg: &mut DrawColor,
    draw_text: &mut DrawText,
    cx: &mut Cx2d,
    x: f64,
    y: f64,
    width: f64,
    panel: &PanelSpec,
) {
    let rect = Rect {
        pos: DVec2 { x, y },
        size: DVec2 {
            x: width,
            y: CARD_HEIGHT,
        },
    };
    draw_card(draw_bg, cx, rect, surface_color());
    draw_rect(
        draw_bg,
        cx,
        Rect {
            pos: DVec2 {
                x: x + 16.0,
                y: y + 18.0,
            },
            size: DVec2 { x: 34.0, y: 4.0 },
        },
        tone_color(panel.tone),
    );
    draw_label(
        draw_text,
        cx,
        x + 16.0,
        y + 34.0,
        panel.title,
        11.0,
        secondary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        x + 16.0,
        y + 66.0,
        panel.metric,
        24.0,
        primary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        x + 16.0,
        y + 106.0,
        panel.detail,
        11.0,
        muted_text_color(),
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_sidebar_text(draw_text: &mut DrawText, cx: &mut Cx2d, rect: Rect) {
    let sidebar = ShellMetrics::sidebar_rect(rect);
    draw_label(
        draw_text,
        cx,
        sidebar.pos.x + 22.0,
        sidebar.pos.y + 24.0,
        "velvet-ballastics",
        17.0,
        primary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        sidebar.pos.x + 22.0,
        sidebar.pos.y + 52.0,
        "local-first workflow flight recorder",
        10.5,
        muted_text_color(),
    );
    draw_nav_text(draw_text, cx, rect, "Overview", 0);
    draw_nav_text(draw_text, cx, rect, "Workflow Graph", 1);
    draw_nav_text(draw_text, cx, rect, "Executions", 2);
    draw_nav_text(draw_text, cx, rect, "Verification", 3);
    draw_nav_text(draw_text, cx, rect, "Replay", 4);
    draw_nav_text(draw_text, cx, rect, "Incidents", 5);
    draw_nav_text(draw_text, cx, rect, "Actions", 6);
    draw_nav_text(draw_text, cx, rect, "Storage", 7);
    draw_nav_text(draw_text, cx, rect, "AI Context", 8);
    draw_nav_text(draw_text, cx, rect, "Settings", 9);
    draw_label(
        draw_text,
        cx,
        sidebar.pos.x + 22.0,
        sidebar.pos.y + sidebar.size.y - 58.0,
        "local server online",
        11.0,
        success_color(),
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_top_bar_text(
    draw_bg: &mut DrawColor,
    draw_text: &mut DrawText,
    cx: &mut Cx2d,
    rect: Rect,
    app_state: &AppState,
) {
    let top = ShellMetrics::top_bar_rect(rect);
    let (chip_one, chip_two) = app_state.screen_status_chips();
    draw_label(
        draw_text,
        cx,
        top.pos.x + 22.0,
        top.pos.y + 16.0,
        app_state.screen_title(),
        19.0,
        primary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        top.pos.x + 22.0,
        top.pos.y + 46.0,
        app_state.screen_subtitle(),
        11.5,
        secondary_text_color(),
    );
    draw_chip(
        draw_bg,
        draw_text,
        cx,
        Rect {
            pos: DVec2 {
                x: top.pos.x + top.size.x - 450.0,
                y: top.pos.y + 22.0,
            },
            size: DVec2 { x: 116.0, y: 28.0 },
        },
        chip_one,
        panel_color(),
        primary_blue_color(),
    );
    draw_chip(
        draw_bg,
        draw_text,
        cx,
        Rect {
            pos: DVec2 {
                x: top.pos.x + top.size.x - 320.0,
                y: top.pos.y + 22.0,
            },
            size: DVec2 { x: 116.0, y: 28.0 },
        },
        chip_two,
        panel_color(),
        success_color(),
    );
    draw_button(
        draw_bg,
        draw_text,
        cx,
        top.pos.x + top.size.x - 185.0,
        top.pos.y + 18.0,
        "Verify",
        primary_blue_color(),
    );
    draw_button(
        draw_bg,
        draw_text,
        cx,
        top.pos.x + top.size.x - 94.0,
        top.pos.y + 18.0,
        "Submit",
        warning_color(),
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_nav_text(draw_text: &mut DrawText, cx: &mut Cx2d, rect: Rect, label: &str, row: u32) {
    let row_rect = SidebarLayout::from_rect(rect).row_rect(row);
    draw_label(
        draw_text,
        cx,
        row_rect.pos.x + 14.0,
        row_rect.pos.y + 10.0,
        label,
        11.5,
        secondary_text_color(),
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_nav_row(
    draw_nav: &mut DrawColor,
    cx: &mut Cx2d,
    rect: Rect,
    app_state: &AppState,
    screen: Screen,
    row: u32,
) {
    let row_rect = SidebarLayout::from_rect(rect).row_rect(row);
    if app_state.current_screen() == screen {
        draw_rect(draw_nav, cx, row_rect, panel_color());
        draw_rect(
            draw_nav,
            cx,
            Rect {
                pos: row_rect.pos,
                size: DVec2 {
                    x: 4.0,
                    y: row_rect.size.y,
                },
            },
            accent_from_rgba(screen.nav_color()),
        );
    }
}

#[allow(elided_lifetimes_in_paths)]
fn draw_panel_lines(draw_text: &mut DrawText, cx: &mut Cx2d, panel: Rect) {
    draw_label(
        draw_text,
        cx,
        panel.pos.x + 20.0,
        panel.pos.y + 64.0,
        "Start -> classify -> route_issue -> create_issue -> build_result",
        12.0,
        secondary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        panel.pos.x + 20.0,
        panel.pos.y + 100.0,
        "Animated edge packets and selected-node glow are shader-driven follow-up work.",
        11.0,
        muted_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        panel.pos.x + 20.0,
        panel.pos.y + 136.0,
        "This shell keeps retained graph/layout data wiring intact for the next pass.",
        11.0,
        muted_text_color(),
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_inspector_lines(draw_text: &mut DrawText, cx: &mut Cx2d, panel: Rect) {
    draw_label(
        draw_text,
        cx,
        panel.pos.x + 20.0,
        panel.pos.y + 64.0,
        "Action ID: act_918cb7v4",
        12.0,
        secondary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        panel.pos.x + 20.0,
        panel.pos.y + 94.0,
        "Idempotency: issue_triage:8421",
        12.0,
        secondary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        panel.pos.x + 20.0,
        panel.pos.y + 124.0,
        "Taint: clean input / clean output",
        12.0,
        success_color(),
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_transport(draw_bg: &mut DrawColor, draw_text: &mut DrawText, cx: &mut Cx2d, content: Rect) {
    let x = content.pos.x + TransportLayout::START_X_OFFSET;
    let y = content.pos.y + TransportLayout::TRANSPORT_Y_OFFSET;
    draw_button(draw_bg, draw_text, cx, x, y, "|<", panel_color());
    draw_button(draw_bg, draw_text, cx, x + 44.0, y, "<", panel_color());
    draw_button(
        draw_bg,
        draw_text,
        cx,
        x + 88.0,
        y,
        "Play",
        primary_blue_color(),
    );
    draw_button(draw_bg, draw_text, cx, x + 170.0, y, ">", panel_color());
    draw_button(draw_bg, draw_text, cx, x + 214.0, y, ">|", panel_color());
}

#[allow(elided_lifetimes_in_paths)]
fn draw_shortcuts(draw_bg: &mut DrawColor, draw_text: &mut DrawText, cx: &mut Cx2d, rect: Rect) {
    let panel = Rect {
        pos: DVec2 {
            x: rect.pos.x + rect.size.x - 360.0,
            y: rect.pos.y + 120.0,
        },
        size: DVec2 { x: 300.0, y: 118.0 },
    };
    draw_card(draw_bg, cx, panel, surface_color());
    draw_label(
        draw_text,
        cx,
        panel.pos.x + 18.0,
        panel.pos.y + 18.0,
        "Shortcuts",
        15.0,
        primary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        panel.pos.x + 18.0,
        panel.pos.y + 50.0,
        "Space: replay play/pause",
        11.0,
        secondary_text_color(),
    );
    draw_label(
        draw_text,
        cx,
        panel.pos.x + 18.0,
        panel.pos.y + 76.0,
        "+/-: workflow zoom",
        11.0,
        secondary_text_color(),
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_button(
    draw_bg: &mut DrawColor,
    draw_text: &mut DrawText,
    cx: &mut Cx2d,
    x: f64,
    y: f64,
    label: &str,
    color: Vec4f,
) {
    let button = Rect {
        pos: DVec2 { x, y },
        size: DVec2 {
            x: TransportLayout::BTN_WIDTH,
            y: 34.0,
        },
    };
    draw_rect(draw_bg, cx, button, color);
    draw_label(
        draw_text,
        cx,
        x + 15.0,
        y + 10.0,
        label,
        11.0,
        primary_text_color(),
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_chip(
    draw_bg: &mut DrawColor,
    draw_text: &mut DrawText,
    cx: &mut Cx2d,
    chip: Rect,
    label: &str,
    bg: Vec4f,
    text: Vec4f,
) {
    draw_rect(draw_bg, cx, chip, bg);
    draw_rect(
        draw_bg,
        cx,
        Rect {
            pos: chip.pos,
            size: DVec2 {
                x: 3.0,
                y: chip.size.y,
            },
        },
        text,
    );
    draw_label(
        draw_text,
        cx,
        chip.pos.x + 12.0,
        chip.pos.y + 8.0,
        label,
        10.0,
        text,
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_card(draw_bg: &mut DrawColor, cx: &mut Cx2d, rect: Rect, fill: Vec4f) {
    draw_rect(draw_bg, cx, rect, border_color());
    draw_inset_rect(draw_bg, cx, rect, fill, ShellMetrics::HAIRLINE);
}

#[allow(elided_lifetimes_in_paths)]
fn draw_accent(draw_bg: &mut DrawColor, cx: &mut Cx2d, rect: Rect, color: [f32; 4]) {
    draw_rect(
        draw_bg,
        cx,
        Rect {
            pos: rect.pos,
            size: DVec2 {
                x: 5.0,
                y: rect.size.y,
            },
        },
        accent_from_rgba(color),
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_inset_rect(draw_bg: &mut DrawColor, cx: &mut Cx2d, rect: Rect, color: Vec4f, inset: f64) {
    draw_rect(
        draw_bg,
        cx,
        Rect {
            pos: DVec2 {
                x: rect.pos.x + inset,
                y: rect.pos.y + inset,
            },
            size: DVec2 {
                x: rect.size.x - (inset * 2.0),
                y: rect.size.y - (inset * 2.0),
            },
        },
        color,
    );
}

#[allow(elided_lifetimes_in_paths)]
fn draw_rect(draw_bg: &mut DrawColor, cx: &mut Cx2d, rect: Rect, color: Vec4f) {
    draw_bg.color = color;
    draw_bg.draw_abs(cx, rect);
}

#[allow(elided_lifetimes_in_paths)]
fn draw_label(
    draw_text: &mut DrawText,
    cx: &mut Cx2d,
    x: f64,
    y: f64,
    text: &str,
    size: f32,
    color: Vec4f,
) {
    draw_text.text_style.font_size = size;
    draw_text.color = color;
    draw_text.draw_abs(cx, DVec2 { x, y }, text);
}

fn tone_color(tone: Tone) -> Vec4f {
    match tone {
        Tone::Neutral => muted_text_color(),
        Tone::Blue => primary_blue_color(),
        Tone::Green => success_color(),
        Tone::Purple => accent_from_rgba([0.431, 0.321, 0.898, 1.0]),
        Tone::Warning => warning_color(),
        Tone::Failure => failure_color(),
    }
}

fn usize_to_f64(value: usize) -> f64 {
    match u32::try_from(value) {
        Ok(converted) => f64::from(converted),
        Err(_) => f64::from(u32::MAX),
    }
}
