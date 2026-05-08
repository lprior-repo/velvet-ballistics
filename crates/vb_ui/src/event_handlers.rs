#![forbid(unsafe_code)]
//! Event handlers for the Mission Control UI.
//!
//! Contains navigation and transport (playback) event handlers extracted
//! from main.rs to satisfy Farley constraints (each function ≤ 25 lines).

use crate::domain::{IpcCleanCycles, TabOffsets, TransportLayout};
use makepad_widgets::*;
use vb_ui::app_state::{AppState, Screen};
use vb_ui::ipc_wiring::{IpcAppWiring, WiringError};
use vb_ui::replay::transport::TransportState;

/// Detects which nav tab (if any) was hit by the finger event.
fn detect_nav_tab_hit(hit: &Hit, rect: &Rect) -> Option<usize> {
    let Hit::FingerDown(fe) = hit else {
        return None;
    };
    if !fe.is_primary_hit() {
        return None;
    }

    let tab_y = rect.pos.y + TabOffsets::HEADER_HEIGHT;
    if fe.abs.y < tab_y || fe.abs.y >= tab_y + TabOffsets::TAB_HEIGHT {
        return None;
    }

    let offsets = TabOffsets::new();
    let rel_x = fe.abs.x - rect.pos.x;
    offsets
        .0
        .iter()
        .position(|&offset| rel_x >= offset && rel_x < offset + TabOffsets::TAB_WIDTH)
}

/// Converts a tab index to its corresponding screen, or None if invalid.
fn resolve_nav_tab_to_screen(tab_idx: usize) -> Option<Screen> {
    match tab_idx {
        0 => Some(Screen::RunReplay),
        1 => Some(Screen::Verification),
        2 => Some(Screen::SystemOverview),
        3 => Some(Screen::WorkflowGraph),
        4 => Some(Screen::IncidentConsole),
        _ => None,
    }
}

/// Handles navigation tab clicks, switching the active screen.
pub(crate) fn handle_nav(app_state: &mut AppState, rect: &Rect, hit: &Hit) {
    let Some(idx) = detect_nav_tab_hit(hit, rect) else {
        return;
    };
    let Some(screen) = resolve_nav_tab_to_screen(idx) else {
        return;
    };
    app_state.switch_screen(screen);
}

/// Detects which transport button (if any) was hit by the finger event.
fn detect_transport_button_hit(hit: &Hit, rect: &Rect, layout: &TransportLayout) -> Option<usize> {
    let Hit::FingerDown(fe) = hit else {
        return None;
    };
    if !fe.is_primary_hit() {
        return None;
    }

    if fe.abs.y < layout.transport_y_offset
        || fe.abs.y >= layout.transport_y_offset + layout.transport_height
    {
        return None;
    }

    let transport_start_x = rect.pos.x + layout.start_x_offset;
    let rel_x = fe.abs.x - transport_start_x;
    let positions = layout.button_positions(transport_start_x);

    positions
        .iter()
        .position(|&btn_x| rel_x >= btn_x && rel_x < btn_x + layout.btn_width)
}

/// Applies the transport button action to the replay state.
fn apply_transport_button_action(app_state: &mut AppState, button_idx: usize) {
    match button_idx {
        0 => jump_to_start(app_state),
        1 => step_backward(app_state),
        2 => toggle_play_pause(app_state),
        3 => jump_to_end(app_state),
        _ => {}
    }
}

fn jump_to_start(app_state: &mut AppState) {
    app_state.replay.playback_position = 0;
    app_state.replay.transport_state = TransportState::Idle;
}

fn step_backward(app_state: &mut AppState) {
    app_state.replay.playback_position =
        app_state.replay.playback_position.saturating_sub(1);
    app_state.replay.transport_state = TransportState::Paused;
}

fn toggle_play_pause(app_state: &mut AppState) {
    app_state.replay.transport_state = if app_state.replay.transport_state.is_playing() {
        TransportState::Paused
    } else {
        TransportState::Playing { next_tick_at: 0 }
    };
}

fn jump_to_end(app_state: &mut AppState) {
    app_state.replay.playback_position = app_state.replay.total_events.saturating_sub(1);
    app_state.replay.transport_state = TransportState::Idle;
}

/// Handles transport (playback) bar button clicks.
pub(crate) fn handle_transport(app_state: &mut AppState, rect: &Rect, hit: &Hit) {
    let layout = TransportLayout::from_rect(rect);
    let Some(btn_idx) = detect_transport_button_hit(hit, rect, &layout) else {
        return;
    };
    apply_transport_button_action(app_state, btn_idx);
}

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
    app_state: &mut AppState,
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

/// Routes IPC wiring events into an IpcChanges summary.
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
    app_state: &mut AppState,
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
