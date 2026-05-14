#![forbid(unsafe_code)]
#![allow(clippy::arithmetic_side_effects)]
//! Event handlers for the Mission Control UI.
//!
//! Emits Makepad actions for navigation and transport controls.
//! State mutation happens in `MatchEvent::handle_actions` in `main.rs`.

use crate::domain::{IpcCleanCycles, SidebarLayout, TransportLayout};
use makepad_widgets::*;
use vb_ui::app_state::Screen;
use vb_ui::ipc_wiring::{IpcAppWiring, WiringError};

// ---------------------------------------------------------------------------
// Action types
// ---------------------------------------------------------------------------

/// Transport control button kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransportControlKind {
    /// Jump to the first event.
    JumpToStart,
    /// Step backward by one event.
    StepBackward,
    /// Step forward by one event.
    StepForward,
    /// Toggle between play and pause.
    TogglePlayPause,
    /// Jump to the last event.
    JumpToEnd,
}

/// Actions emitted by the `VbApp` widget.
#[derive(Clone, Debug, Default)]
pub(crate) enum VbAction {
    /// No operation (default).
    #[default]
    NoOp,
    /// Switch to a different screen.
    SwitchScreen(Screen),
    /// Transport control button was pressed.
    TransportControl(TransportControlKind),
    /// Escape key was pressed.
    Escape,
    /// Toggle shortcuts help overlay.
    ToggleShortcuts,
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

/// Detects which sidebar nav row (if any) was hit by the finger event.
fn detect_nav_row_hit(hit: &Hit, rect: &Rect) -> Option<u32> {
    let Hit::FingerDown(fe) = hit else {
        return None;
    };
    if !fe.is_primary_hit() {
        return None;
    }

    let layout = SidebarLayout::from_rect(*rect);
    if fe.abs.x < layout.x || fe.abs.x >= layout.x + layout.row_width {
        return None;
    }

    let rel_y = fe.abs.y - layout.nav_y;
    row_from_relative_y(rel_y, &layout)
}

fn row_from_relative_y(rel_y: f64, layout: &SidebarLayout) -> Option<u32> {
    if row_contains(rel_y, layout, 0) {
        Some(0)
    } else if row_contains(rel_y, layout, 1) {
        Some(1)
    } else if row_contains(rel_y, layout, 2) {
        Some(2)
    } else if row_contains(rel_y, layout, 3) {
        Some(3)
    } else if row_contains(rel_y, layout, 4) {
        Some(4)
    } else if row_contains(rel_y, layout, 5) {
        Some(5)
    } else if row_contains(rel_y, layout, 6) {
        Some(6)
    } else if row_contains(rel_y, layout, 7) {
        Some(7)
    } else {
        None
    }
}

fn row_contains(rel_y: f64, layout: &SidebarLayout, row: u32) -> bool {
    let top = f64::from(row) * (layout.row_height + layout.row_gap);
    rel_y >= top && rel_y < top + layout.row_height
}

/// Converts a sidebar row to its corresponding screen, or `None` if invalid.
fn resolve_nav_row_to_screen(row: u32) -> Option<Screen> {
    match row {
        0 => Some(Screen::ExecutionOverview),
        1 => Some(Screen::WorkflowGraphAuthoring),
        2 => Some(Screen::ExecutionDetailsGraph),
        3 => Some(Screen::VerificationCertificate),
        4 => Some(Screen::ReplayTheater),
        5 => Some(Screen::IncidentFailureConsole),
        6 => Some(Screen::ActionRegistry),
        7 => Some(Screen::StorageDoctorAiContext),
        _ => None,
    }
}

/// Emits a `SwitchScreen` action when a sidebar nav row is clicked.
pub(crate) fn handle_nav(cx: &mut Cx, uid: WidgetUid, rect: &Rect, hit: &Hit) {
    let Some(row) = detect_nav_row_hit(hit, rect) else {
        return;
    };
    let Some(screen) = resolve_nav_row_to_screen(row) else {
        return;
    };
    cx.widget_action(uid, VbAction::SwitchScreen(screen));
}

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

/// Detects which transport button (if any) was hit by the finger event.
fn detect_transport_button_hit(hit: &Hit, layout: &TransportLayout) -> Option<usize> {
    let Hit::FingerDown(fe) = hit else {
        return None;
    };
    if !fe.is_primary_hit() {
        return None;
    }

    if fe.abs.y < layout.transport_y || fe.abs.y >= layout.transport_y + layout.transport_height {
        return None;
    }

    let rel_x = fe.abs.x - layout.transport_x;
    let positions = layout.button_positions();

    positions
        .iter()
        .position(|&btn_x| rel_x >= btn_x && rel_x < btn_x + layout.btn_width)
}

/// Maps a button index to its transport control kind.
fn resolve_button_index_to_control(button_idx: usize) -> Option<TransportControlKind> {
    match button_idx {
        0 => Some(TransportControlKind::JumpToStart),
        1 => Some(TransportControlKind::StepBackward),
        2 => Some(TransportControlKind::TogglePlayPause),
        3 => Some(TransportControlKind::StepForward),
        4 => Some(TransportControlKind::JumpToEnd),
        _ => None,
    }
}

/// Emits a `TransportControl` action when a transport button is clicked.
pub(crate) fn handle_transport(
    cx: &mut Cx,
    uid: WidgetUid,
    rect: &Rect,
    hit: &Hit,
    app_state: &vb_ui::app_state::AppState,
) {
    if app_state.current_screen != Screen::ReplayTheater {
        return;
    }
    let layout = TransportLayout::from_rect(rect);
    let Some(btn_idx) = detect_transport_button_hit(hit, &layout) else {
        return;
    };
    let Some(control) = resolve_button_index_to_control(btn_idx) else {
        return;
    };
    cx.widget_action(uid, VbAction::TransportControl(control));
}

/// Emits transport control or escape actions when keyboard keys are pressed.
pub(crate) fn handle_keyboard(
    cx: &mut Cx,
    uid: WidgetUid,
    event: &Event,
    app_state: &vb_ui::app_state::AppState,
    workflow_canvas: &mut Option<vb_ui::workflow::WorkflowCanvas>,
) {
    let Event::KeyDown(kde) = event else {
        return;
    };
    match kde.key_code {
        KeyCode::Space if app_state.current_screen == Screen::ReplayTheater => {
            cx.widget_action(
                uid,
                VbAction::TransportControl(TransportControlKind::TogglePlayPause),
            );
        }
        KeyCode::Equals | KeyCode::Minus
            if app_state.current_screen == vb_ui::app_state::Screen::WorkflowGraphAuthoring =>
        {
            if let Some(canvas) = workflow_canvas {
                const ZOOM_FACTOR: f64 = 1.25;
                if kde.key_code == KeyCode::Equals {
                    canvas.zoom_in(ZOOM_FACTOR);
                } else {
                    canvas.zoom_out(ZOOM_FACTOR);
                }
            }
        }
        KeyCode::Escape => {
            cx.widget_action(uid, VbAction::Escape);
        }
        KeyCode::Slash if kde.modifiers.shift => {
            cx.widget_action(uid, VbAction::ToggleShortcuts);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// IPC wiring
// ---------------------------------------------------------------------------
// IPC wiring
// ---------------------------------------------------------------------------

/// Formats a wiring error into a user-facing message.
fn format_wiring_error(err: &WiringError) -> String {
    match err {
        WiringError::ConnectionFailed(detail) => {
            format!("IPC connection failed: {detail}")
        }
        WiringError::IpcError(detail) => {
            format!("IPC error: {detail}")
        }
    }
}

/// Handles IPC errors: records error or increments clean cycles.
fn handle_ipc_errors(
    errors: &[WiringError],
    app_state: &mut vb_ui::app_state::AppState,
    ipc_clean_cycles: &mut IpcCleanCycles,
) {
    if errors.is_empty() {
        if app_state.last_ipc_error.is_some() {
            ipc_clean_cycles.increment();
            if ipc_clean_cycles.is_resolved() {
                app_state.last_ipc_error = None;
                ipc_clean_cycles.reset();
            }
        }
    } else {
        ipc_clean_cycles.reset();
        if let Some(err) = errors.first() {
            app_state.last_ipc_error = Some(format_wiring_error(err));
        }
    }
}

/// Routes IPC wiring events into an `IpcChanges` summary.
fn route_ipc_events(wiring_events: &vb_ui::ipc_wiring::WiringEvents) -> IpcChanges {
    IpcChanges {
        metrics_updated: wiring_events.metrics_updated,
        connection_changed: wiring_events.connection_changed,
        health_checked: wiring_events.health_checked,
        run_list_updated: wiring_events.run_list_updated,
        verification_updated: wiring_events.verification_updated,
        taint_report_updated: wiring_events.taint_report_updated,
        run_accepted: wiring_events.run_accepted,
        run_cancelled: wiring_events.run_cancelled,
        events_arrived: wiring_events.events_arrived,
        trace_drained: wiring_events.trace_drained,
        inspected: wiring_events.inspected,
        workflow_graph_updated: wiring_events.workflow_graph_updated,
        has_errors: !wiring_events.errors.is_empty(),
    }
}

/// Polls IPC wiring and returns whether metrics/changes require a sync.
pub(crate) fn poll_ipc_and_detect_changes(
    ipc_wiring: &mut IpcAppWiring,
    app_state: &mut vb_ui::app_state::AppState,
    ipc_clean_cycles: &mut IpcCleanCycles,
) -> IpcChanges {
    let wiring_events = ipc_wiring.poll(app_state);
    handle_ipc_errors(&wiring_events.errors, app_state, ipc_clean_cycles);
    route_ipc_events(&wiring_events)
}

/// Aggregates which UI state groups need syncing after an IPC poll.
#[derive(Debug, Clone, Default)]
pub(crate) struct IpcChanges {
    pub metrics_updated: bool,
    pub connection_changed: bool,
    pub health_checked: bool,
    pub run_list_updated: bool,
    pub verification_updated: bool,
    pub taint_report_updated: bool,
    pub run_accepted: bool,
    pub run_cancelled: bool,
    pub events_arrived: bool,
    pub trace_drained: bool,
    pub inspected: bool,
    pub workflow_graph_updated: bool,
    pub has_errors: bool,
}

impl IpcChanges {
    pub(crate) fn needs_system_sync(&self) -> bool {
        self.metrics_updated
            || self.connection_changed
            || self.health_checked
            || self.run_list_updated
            || self.has_errors
    }

    pub(crate) fn needs_verify_sync(&self) -> bool {
        self.verification_updated || self.taint_report_updated
    }

    pub(crate) fn needs_replay_sync(&self) -> bool {
        self.run_accepted
            || self.run_cancelled
            || self.events_arrived
            || self.trace_drained
            || self.inspected
    }

    pub(crate) fn needs_workflow_sync(&self) -> bool {
        self.workflow_graph_updated
    }
}
