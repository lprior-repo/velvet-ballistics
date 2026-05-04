#![forbid(unsafe_code)]
//! App-level IPC wiring that routes `IpcReply` events to `AppState`.
//!
//! This module sits between the low-level [`IpcBridge`] (which owns the
//! background IPC thread and channels) and the Makepad [`AppState`] struct.
//! Call [`IpcAppWiring::poll`] from the Makepad render loop (e.g.
//! `handle_next_frame`) to drain pending replies and apply them to the
//! appropriate screen data.

use std::path::PathBuf;

use vb_core::ids::RunId;

use crate::app_state::{AppState, HealthLevel};
use crate::ipc_bridge::{IpcBridge, IpcReply, IpcRequest};
use crate::theme::colors;

// ---------------------------------------------------------------------------
// Wiring struct
// ---------------------------------------------------------------------------

/// Owns the [`IpcBridge`] and translates IPC replies into `AppState` mutations.
///
/// Typical lifecycle:
/// 1. Create with [`IpcAppWiring::new`].
/// 2. Call [`IpcAppWiring::connect`] when the app starts or the user selects
///    a socket path.
/// 3. Call [`IpcAppWiring::poll`] every frame from the Makepad render loop.
/// 4. Inspect the returned [`WiringEvents`] to decide whether to redraw.
pub struct IpcAppWiring {
    bridge: IpcBridge,
}

impl Default for IpcAppWiring {
    fn default() -> Self {
        Self::new()
    }
}

impl IpcAppWiring {
    /// Creates a new wiring with a fresh IPC bridge.
    pub fn new() -> Self {
        Self {
            bridge: IpcBridge::new(),
        }
    }

    /// Initiates a connection to the given socket path.
    pub fn connect(&self, socket_path: PathBuf) -> Result<(), String> {
        self.bridge
            .send(IpcRequest::Connect { socket_path })
            .map_err(|e| format!("IPC connect request failed: {e}"))
    }

    /// Initiates a disconnection.
    pub fn disconnect(&self) -> Result<(), String> {
        self.bridge
            .send(IpcRequest::Disconnect)
            .map_err(|e| format!("IPC disconnect request failed: {e}"))
    }

    /// Requests a health check from the server.
    pub fn health(&self) -> Result<(), String> {
        self.bridge
            .send(IpcRequest::Health)
            .map_err(|e| format!("IPC health request failed: {e}"))
    }

    /// Requests an inspect for the given run.
    pub fn inspect_run(&self, run_id: RunId) -> Result<(), String> {
        self.bridge
            .send(IpcRequest::InspectRun { run_id })
            .map_err(|e| format!("IPC inspect-run request failed: {e}"))
    }

    /// Requests metrics from the server.
    pub fn drain_trace(&self, run_id: RunId, max_records: u32) -> Result<(), String> {
        self.bridge
            .send(IpcRequest::DrainTrace {
                run_id,
                max_records,
            })
            .map_err(|e| format!("IPC drain-trace request failed: {e}"))
    }

    /// Returns whether the underlying bridge is connected.
    pub fn is_connected(&self) -> bool {
        self.bridge.is_connected()
    }

    /// Polls the IPC bridge and routes replies into `app_state`.
    ///
    /// Returns a [`WiringEvents`] summarising what changed so the caller can
    /// decide which screens need redrawing.
    pub fn poll(&mut self, app_state: &mut AppState) -> WiringEvents {
        let replies = self.bridge.poll();
        let mut events = WiringEvents::default();

        for reply in replies {
            self.route_reply(reply, app_state, &mut events);
        }

        events
    }

    // -- Internal routing ---------------------------------------------------

    fn route_reply(&self, reply: IpcReply, app_state: &mut AppState, events: &mut WiringEvents) {
        match reply {
            IpcReply::Connected => {
                app_state.connected = true;
                events.connection_changed = true;
                events.connected = true;
            }
            IpcReply::Disconnected => {
                app_state.connected = false;
                events.connection_changed = true;
                events.disconnected = true;
            }
            IpcReply::ConnectionFailed(err) => {
                app_state.connected = false;
                events.connection_changed = true;
                events.errors.push(WiringError::ConnectionFailed(err));
            }
            IpcReply::RunAccepted(run_id) => {
                app_state.selected_run_id = Some(run_id.get());
                events.run_accepted = true;
            }
            IpcReply::RunCancelled(run_id) => {
                if app_state.selected_run_id == Some(run_id.get()) {
                    app_state.selected_run_id = None;
                }
                events.run_cancelled = true;
            }
            IpcReply::Inspected(response) => {
                self.route_inspected(response, app_state, events);
            }
            IpcReply::Events(response) => {
                // Events replies are consumed by the replay controller
                // directly. We record that events arrived so the caller can
                // delegate to the replay sub-system.
                let _ = response;
                events.events_arrived = true;
            }
            IpcReply::TraceCount(count) => {
                let _ = count;
                events.trace_drained = true;
            }
            IpcReply::Healthy => {
                app_state.system.overall_health = HealthLevel::Healthy;
                events.health_checked = true;
            }
            IpcReply::ShuttingDown => {
                app_state.connected = false;
                events.connection_changed = true;
                events.shutting_down = true;
            }
            IpcReply::Error(err) => {
                events.errors.push(WiringError::IpcError(err));
            }
            IpcReply::NotImplemented(msg) => {
                events
                    .errors
                    .push(WiringError::IpcError(format!("Not implemented: {msg}")));
            }
        }
    }

    fn route_inspected(
        &self,
        response: vb_ipc::server::IpcResponse,
        app_state: &mut AppState,
        events: &mut WiringEvents,
    ) {
        match response {
            vb_ipc::server::IpcResponse::Inspected { run_id } => {
                app_state.selected_run_id = Some(run_id);
                events.inspected = true;
            }
            vb_ipc::server::IpcResponse::RunList { runs } => {
                let active_count = u32::try_from(runs.len()).unwrap_or(u32::MAX);
                app_state.system.total_active_runs = active_count;
                events.run_list_updated = true;
            }
            vb_ipc::server::IpcResponse::Metrics(metrics) => {
                let shard_count = u32::try_from(metrics.shards.len()).unwrap_or(u32::MAX);
                app_state.system.shard_count = shard_count;
                app_state.system.total_active_runs = metrics.totals.runs_active;

                let total_queue = metrics
                    .shards
                    .iter()
                    .fold(0u32, |acc, s| acc.saturating_add(s.ready_queue_depth));
                app_state.system.total_queue_depth = total_queue;

                // Determine health: degraded if any shard has high queue
                // pressure, critical if any shard is severely overloaded.
                let worst_health = metrics.shards.iter().fold(
                    HealthLevel::Healthy,
                    |current, shard| {
                        let frame_pct = frame_pool_used_pct(shard);
                        if shard.ready_queue_depth > 50 || frame_pct > 90 {
                            HealthLevel::Critical
                        } else if shard.ready_queue_depth > 20 || frame_pct > 75 {
                            match current {
                                HealthLevel::Critical => HealthLevel::Critical,
                                _ => HealthLevel::Degraded,
                            }
                        } else {
                            current
                        }
                    },
                );
                app_state.system.overall_health = worst_health;
                events.metrics_updated = true;
            }
            vb_ipc::server::IpcResponse::VerifyWorkflow { result } => {
                app_state.verification.total_checks = result.total_checks;
                app_state.verification.pass_count = result.pass_count;
                app_state.verification.fail_count = result.fail_count;
                app_state.verification.warn_count = result
                    .total_checks
                    .saturating_sub(result.pass_count)
                    .saturating_sub(result.fail_count);
                app_state.verification.all_clean =
                    result.fail_count == 0 && app_state.verification.warn_count == 0;
                events.verification_updated = true;
            }
            vb_ipc::server::IpcResponse::TaintReport {
                finish_safe,
                ..
            } => {
                if finish_safe {
                    app_state.verification.all_clean = true;
                }
                events.taint_report_updated = true;
            }
            vb_ipc::server::IpcResponse::WorkflowGraph { nodes, .. } => {
                app_state.workflow.node_count =
                    u32::try_from(nodes.len()).unwrap_or(u32::MAX);
                events.workflow_graph_updated = true;
            }
            _ => {
                events
                    .errors
                    .push(WiringError::IpcError("Unexpected inspect response".into()));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the frame pool utilization percentage for a shard.
#[allow(clippy::manual_unwrap_or)]
fn frame_pool_used_pct(shard: &vb_ipc::ShardMetrics) -> u32 {
    let total = shard.frame_pool_total;
    if total == 0 {
        return 0;
    }
    let used = total.saturating_sub(shard.frame_pool_free);
    let product = used.saturating_mul(100);
    match product.checked_div(total) {
        Some(pct) => pct,
        None => u32::MAX,
    }
}

// ---------------------------------------------------------------------------
// Event accumulator
// ---------------------------------------------------------------------------

/// Summary of what changed during a single [`IpcAppWiring::poll`] call.
///
/// The Makepad app can inspect these flags to decide which screens need
/// redrawing, avoiding full redraws when nothing changed.
#[derive(Debug, Default)]
pub struct WiringEvents {
    /// Connection state changed (connected, disconnected, or failed).
    pub connection_changed: bool,
    /// Successfully connected.
    pub connected: bool,
    /// Disconnected (clean or server shutdown).
    pub disconnected: bool,
    /// Server is shutting down.
    pub shutting_down: bool,
    /// A run was accepted.
    pub run_accepted: bool,
    /// A run was cancelled.
    pub run_cancelled: bool,
    /// An inspect reply was processed.
    pub inspected: bool,
    /// Journal events arrived (replay subsystem should pick them up).
    pub events_arrived: bool,
    /// Trace drain completed.
    pub trace_drained: bool,
    /// Health check completed.
    pub health_checked: bool,
    /// System metrics were updated.
    pub metrics_updated: bool,
    /// Run list was updated.
    pub run_list_updated: bool,
    /// Verification result was updated.
    pub verification_updated: bool,
    /// Taint report was updated.
    pub taint_report_updated: bool,
    /// Workflow graph was updated.
    pub workflow_graph_updated: bool,
    /// Errors accumulated during this poll cycle.
    pub errors: Vec<WiringError>,
}

impl WiringEvents {
    /// Returns the accent color to use for connection status indicators.
    ///
    /// Uses the cyberpunk palette:
    /// - Connected: neon cyan
    /// - Disconnected / errors: neon red
    /// - Shutting down: neon yellow (warning)
    /// - Default (idle): dim text
    pub fn connection_status_color(&self) -> [f32; 4] {
        if self.errors.is_empty() && self.connected {
            colors::neon::CYAN
        } else if self.disconnected || !self.errors.is_empty() {
            colors::neon::RED
        } else if self.shutting_down {
            colors::neon::YELLOW
        } else {
            colors::text::DIM
        }
    }

    /// Returns a human-readable connection status string.
    pub fn connection_status_text(&self) -> &'static str {
        if self.connected {
            "CONNECTED"
        } else if self.disconnected {
            "DISCONNECTED"
        } else if self.shutting_down {
            "SHUTTING DOWN"
        } else if !self.errors.is_empty() {
            "ERROR"
        } else {
            "IDLE"
        }
    }

    /// Returns true if any event was produced (the UI should consider
    /// redrawing).
    pub fn any_changed(&self) -> bool {
        self.connection_changed
            || self.run_accepted
            || self.run_cancelled
            || self.inspected
            || self.events_arrived
            || self.trace_drained
            || self.health_checked
            || self.metrics_updated
            || self.run_list_updated
            || self.verification_updated
            || self.taint_report_updated
            || self.workflow_graph_updated
            || !self.errors.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during IPC wiring.
#[derive(Debug)]
pub enum WiringError {
    /// Connection attempt failed.
    ConnectionFailed(String),
    /// Generic IPC error.
    IpcError(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiring_new_is_not_connected() {
        let wiring = IpcAppWiring::new();
        assert!(!wiring.is_connected());
    }

    #[test]
    fn poll_with_no_ipc_activity_returns_empty_events() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let events = wiring.poll(&mut state);
        assert!(!events.any_changed());
    }

    #[test]
    fn wiring_events_default_has_no_changes() {
        let events = WiringEvents::default();
        assert!(!events.any_changed());
        assert!(events.errors.is_empty());
    }

    #[test]
    fn wiring_events_connected_color_is_cyan() {
        let events = WiringEvents {
            connected: true,
            ..WiringEvents::default()
        };
        assert_eq!(events.connection_status_color(), colors::neon::CYAN);
    }

    #[test]
    fn wiring_events_disconnected_color_is_red() {
        let events = WiringEvents {
            disconnected: true,
            ..WiringEvents::default()
        };
        assert_eq!(events.connection_status_color(), colors::neon::RED);
    }

    #[test]
    fn wiring_events_shutting_down_color_is_yellow() {
        let events = WiringEvents {
            shutting_down: true,
            ..WiringEvents::default()
        };
        assert_eq!(events.connection_status_color(), colors::neon::YELLOW);
    }

    #[test]
    fn wiring_events_idle_color_is_dim() {
        let events = WiringEvents::default();
        assert_eq!(events.connection_status_color(), colors::text::DIM);
    }

    #[test]
    fn wiring_events_error_color_is_red() {
        let events = WiringEvents {
            errors: vec![WiringError::IpcError("test".into())],
            ..WiringEvents::default()
        };
        assert_eq!(events.connection_status_color(), colors::neon::RED);
    }

    #[test]
    fn connection_status_text_variants() {
        assert_eq!(
            WiringEvents {
                connected: true,
                ..WiringEvents::default()
            }
            .connection_status_text(),
            "CONNECTED"
        );
        assert_eq!(
            WiringEvents {
                disconnected: true,
                ..WiringEvents::default()
            }
            .connection_status_text(),
            "DISCONNECTED"
        );
        assert_eq!(
            WiringEvents {
                shutting_down: true,
                ..WiringEvents::default()
            }
            .connection_status_text(),
            "SHUTTING DOWN"
        );
        assert_eq!(
            WiringEvents {
                errors: vec![WiringError::IpcError("x".into())],
                ..WiringEvents::default()
            }
            .connection_status_text(),
            "ERROR"
        );
        assert_eq!(WiringEvents::default().connection_status_text(), "IDLE");
    }

    #[test]
    fn frame_pool_used_pct_normal() {
        let shard = vb_ipc::ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 25,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        };
        assert_eq!(frame_pool_used_pct(&shard), 75);
    }

    #[test]
    fn frame_pool_used_pct_zero_total() {
        let shard = vb_ipc::ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        };
        assert_eq!(frame_pool_used_pct(&shard), 0);
    }

    #[test]
    fn frame_pool_used_pct_full() {
        let shard = vb_ipc::ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        };
        assert_eq!(frame_pool_used_pct(&shard), 100);
    }
}
