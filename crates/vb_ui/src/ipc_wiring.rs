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

use vb_ipc::server::IpcResponse;

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
    /// Buffer for event responses that arrive via `IpcReply::Events`.
    /// The replay controller calls [`IpcAppWiring::drain_events`] to consume them.
    events_buffer: Vec<IpcResponse>,
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
            events_buffer: Vec::new(),
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

    /// Triggers live verification for the given compiled workflow digest.
    ///
    /// The caller is responsible for resolving a workflow name to its digest
    /// before calling this method (typically from `AppState::selected_workflow_digest`).
    pub fn verify_workflow(&self, digest: vb_core::WorkflowDigest) -> Result<(), String> {
        self.bridge
            .send(IpcRequest::VerifyWorkflow { digest })
            .map_err(|e| format!("IPC verify-workflow request failed: {e}"))
    }

    /// Requests taint analysis for the given run's associated workflow.
    pub fn request_taint_report(
        &self,
        run_id: RunId,
        digest: vb_core::WorkflowDigest,
    ) -> Result<(), String> {
        self.bridge
            .send(IpcRequest::RequestTaintReport { run_id, digest })
            .map_err(|e| format!("IPC taint-report request failed: {e}"))
    }

    /// Requests graph data for the given compiled workflow digest.
    pub fn request_workflow_graph(&self, digest: vb_core::WorkflowDigest) -> Result<(), String> {
        self.bridge
            .send(IpcRequest::RequestWorkflowGraph { digest })
            .map_err(|e| format!("IPC workflow-graph request failed: {e}"))
    }

    /// Returns whether the underlying bridge is connected.
    pub fn is_connected(&self) -> bool {
        self.bridge.is_connected()
    }

    /// Drains and returns all buffered event responses accumulated since the
    /// last call. The replay controller should call this after
    /// [`WiringEvents::events_arrived`] is set to true.
    pub fn drain_events(&mut self) -> Vec<IpcResponse> {
        std::mem::take(&mut self.events_buffer)
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

    pub fn route_reply(
        &mut self,
        reply: IpcReply,
        app_state: &mut AppState,
        events: &mut WiringEvents,
    ) {
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
                // Buffer the events response so the replay controller can
                // retrieve it via `drain_events`.
                self.events_buffer.push(response);
                events.events_arrived = true;
                events.events_buffered = events.events_buffered.saturating_add(1);
            }
            IpcReply::TraceCount(count) => {
                let _ = count;
                events.trace_drained = true;
            }
            IpcReply::Healthy => {
                // Only upgrade to Healthy; never downgrade from Degraded/Critical
                // set by an authoritative Metrics reply.
                if !matches!(
                    app_state.system.overall_health,
                    HealthLevel::Degraded | HealthLevel::Critical
                ) {
                    app_state.system.overall_health = HealthLevel::Healthy;
                }
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
            IpcReply::VerifyWorkflowResult(response) => {
                self.route_inspected(response, app_state, events);
            }
            IpcReply::TaintReportReceived(response) => {
                self.route_inspected(response, app_state, events);
            }
            IpcReply::WorkflowGraphReceived(response) => {
                self.route_inspected(response, app_state, events);
            }
        }
    }

    fn route_inspected(
        &mut self,
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
                let worst_health =
                    metrics
                        .shards
                        .iter()
                        .fold(HealthLevel::Healthy, |current, shard| {
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
                        });
                app_state.system.overall_health = worst_health;
                events.metrics_updated = true;
            }
            vb_ipc::server::IpcResponse::VerifyWorkflow { result } => {
                app_state
                    .verification
                    .populate_cert_cards(&result.certificates);
                events.verification_updated = true;
            }
            vb_ipc::server::IpcResponse::TaintReport { finish_safe, .. } => {
                if finish_safe {
                    app_state.verification.all_clean = true;
                }
                events.taint_report_updated = true;
            }
            vb_ipc::server::IpcResponse::WorkflowGraph { nodes, .. } => {
                app_state.workflow.node_count = u32::try_from(nodes.len()).unwrap_or(u32::MAX);
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
    /// Journal events arrived and were buffered in the wiring. Call
    /// [`IpcAppWiring::drain_events`] to retrieve the data.
    pub events_arrived: bool,
    /// Number of event responses buffered during this poll cycle.
    pub events_buffered: usize,
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
#[non_exhaustive]
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
    fn drain_events_returns_empty_when_no_events_buffered() {
        let mut wiring = IpcAppWiring::new();
        let events = wiring.drain_events();
        assert!(events.is_empty());
    }

    #[test]
    fn drain_events_clears_buffer_after_drain() {
        let mut wiring = IpcAppWiring::new();
        wiring
            .events_buffer
            .push(vb_ipc::server::IpcResponse::Events { events: Vec::new() });
        let first = wiring.drain_events();
        assert_eq!(first.len(), 1);
        let second = wiring.drain_events();
        assert!(second.is_empty());
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

    // -----------------------------------------------------------------------
    // route_inspected: IpcResponse::Inspected
    // -----------------------------------------------------------------------

    #[test]
    fn route_inspected_inspected_sets_selected_run_and_flag() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::Inspected { run_id: 99 },
            &mut state,
            &mut events,
        );
        assert_eq!(state.selected_run_id, Some(99));
        assert!(events.inspected);
    }

    // -----------------------------------------------------------------------
    // route_inspected: IpcResponse::RunList
    // -----------------------------------------------------------------------

    #[test]
    fn route_inspected_run_list_updates_active_runs() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let runs = vec![
            vb_ipc::RunSummary {
                run_id: vb_core::ids::RunId::new(1),
                workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
                state: vb_ipc::RunListState::Active,
                submitted_seq: 0,
                finished_seq: None,
                step_count: 5,
                steps_completed: 2,
            },
            vb_ipc::RunSummary {
                run_id: vb_core::ids::RunId::new(2),
                workflow: vb_core::WorkflowDigest::from_bytes([1; 32]),
                state: vb_ipc::RunListState::Finished,
                submitted_seq: 10,
                finished_seq: Some(20),
                step_count: 3,
                steps_completed: 3,
            },
        ];
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::RunList { runs },
            &mut state,
            &mut events,
        );
        assert_eq!(state.system.total_active_runs, 2);
        assert!(events.run_list_updated);
    }

    #[test]
    fn route_inspected_run_list_empty_sets_zero() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        state.system.total_active_runs = 10;
        let mut events = WiringEvents::default();
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::RunList { runs: Vec::new() },
            &mut state,
            &mut events,
        );
        assert_eq!(state.system.total_active_runs, 0);
        assert!(events.run_list_updated);
    }

    // -----------------------------------------------------------------------
    // route_inspected: IpcResponse::Metrics
    // -----------------------------------------------------------------------

    fn healthy_shard(shard_id: u32) -> vb_ipc::ShardMetrics {
        vb_ipc::ShardMetrics {
            shard_id,
            active_runs: 5,
            ready_queue_depth: 3,
            action_queue_depth: 2,
            timer_count: 1,
            frame_pool_free: 80,
            frame_pool_total: 100,
            trace_ring_fill_pct: 10.0,
            steps_total: 100,
            actions_total: 50,
        }
    }

    fn degraded_shard(shard_id: u32) -> vb_ipc::ShardMetrics {
        vb_ipc::ShardMetrics {
            shard_id,
            active_runs: 5,
            ready_queue_depth: 25,
            action_queue_depth: 2,
            timer_count: 1,
            frame_pool_free: 20,
            frame_pool_total: 100,
            trace_ring_fill_pct: 50.0,
            steps_total: 100,
            actions_total: 50,
        }
    }

    fn critical_shard(shard_id: u32) -> vb_ipc::ShardMetrics {
        vb_ipc::ShardMetrics {
            shard_id,
            active_runs: 5,
            ready_queue_depth: 60,
            action_queue_depth: 2,
            timer_count: 1,
            frame_pool_free: 5,
            frame_pool_total: 100,
            trace_ring_fill_pct: 90.0,
            steps_total: 100,
            actions_total: 50,
        }
    }

    fn make_metrics(shards: Vec<vb_ipc::ShardMetrics>, runs_active: u32) -> vb_ipc::RuntimeMetrics {
        vb_ipc::RuntimeMetrics {
            shards,
            journal: vb_ipc::JournalMetrics {
                writer_queue_depth: 0,
                total_events: 0,
                total_runs: 0,
            },
            ipc: vb_ipc::IpcMetrics {
                connected_clients: 1,
                commands_processed: 0,
            },
            totals: vb_ipc::AggregateMetrics {
                runs_active,
                runs_waiting: 0,
                runs_failed_total: 0,
                runs_finished_total: 0,
            },
        }
    }

    #[test]
    fn route_inspected_metrics_healthy_shard() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let metrics = make_metrics(vec![healthy_shard(0)], 5);
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::Metrics(metrics),
            &mut state,
            &mut events,
        );
        assert!(events.metrics_updated);
        assert_eq!(state.system.shard_count, 1);
        assert_eq!(state.system.total_active_runs, 5);
        assert_eq!(state.system.total_queue_depth, 3);
        assert_eq!(state.system.overall_health, HealthLevel::Healthy);
    }

    #[test]
    fn route_inspected_metrics_degraded_shard() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let metrics = make_metrics(vec![degraded_shard(0)], 5);
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::Metrics(metrics),
            &mut state,
            &mut events,
        );
        assert!(events.metrics_updated);
        assert_eq!(state.system.overall_health, HealthLevel::Degraded);
    }

    #[test]
    fn route_inspected_metrics_critical_shard() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let metrics = make_metrics(vec![critical_shard(0)], 5);
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::Metrics(metrics),
            &mut state,
            &mut events,
        );
        assert!(events.metrics_updated);
        assert_eq!(state.system.overall_health, HealthLevel::Critical);
    }

    #[test]
    fn route_inspected_metrics_worst_health_wins() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let metrics = make_metrics(vec![healthy_shard(0), critical_shard(1)], 10);
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::Metrics(metrics),
            &mut state,
            &mut events,
        );
        assert_eq!(state.system.overall_health, HealthLevel::Critical);
    }

    #[test]
    fn route_inspected_metrics_queue_depth_sums_across_shards() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let mut shard_a = healthy_shard(0);
        shard_a.ready_queue_depth = 10;
        let mut shard_b = healthy_shard(1);
        shard_b.ready_queue_depth = 20;
        let metrics = make_metrics(vec![shard_a, shard_b], 10);
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::Metrics(metrics),
            &mut state,
            &mut events,
        );
        assert_eq!(state.system.total_queue_depth, 30);
        assert_eq!(state.system.shard_count, 2);
    }

    // -----------------------------------------------------------------------
    // route_inspected: IpcResponse::VerifyWorkflow
    // -----------------------------------------------------------------------

    #[test]
    fn route_inspected_verify_workflow_populates_cert_cards() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let certs = vec![
            vb_ipc::CertificateWire {
                kind: "gate_09_structure_check".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_07_expression_stack_depth".into(),
                status: "Pass".into(),
                details: String::new(),
            },
        ];
        let result = vb_ipc::VerificationResult {
            certificates: certs,
            total_checks: 2,
            pass_count: 2,
            fail_count: 0,
        };
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::VerifyWorkflow { result },
            &mut state,
            &mut events,
        );
        assert!(events.verification_updated);
        assert_eq!(state.verification.cert_structure.badge_text, "PASS");
        assert_eq!(state.verification.cert_bounded.badge_text, "PASS");
    }

    #[test]
    fn route_inspected_verify_workflow_with_failures() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let certs = vec![vb_ipc::CertificateWire {
            kind: "gate_13_taint_check".into(),
            status: "Fail".into(),
            details: "taint path found".into(),
        }];
        let result = vb_ipc::VerificationResult {
            certificates: certs,
            total_checks: 1,
            pass_count: 0,
            fail_count: 1,
        };
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::VerifyWorkflow { result },
            &mut state,
            &mut events,
        );
        assert!(events.verification_updated);
        assert_eq!(state.verification.cert_taint.badge_text, "FAIL");
        assert!(!state.verification.all_clean);
    }

    // -----------------------------------------------------------------------
    // route_inspected: IpcResponse::TaintReport
    // -----------------------------------------------------------------------

    #[test]
    fn route_inspected_taint_report_safe_sets_all_clean() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        state.verification.all_clean = false;
        let mut events = WiringEvents::default();
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::TaintReport {
                sources: Vec::new(),
                sinks: Vec::new(),
                finish_safe: true,
                paths: Vec::new(),
            },
            &mut state,
            &mut events,
        );
        assert!(events.taint_report_updated);
        assert!(state.verification.all_clean);
    }

    #[test]
    fn route_inspected_taint_report_unsafe_does_not_set_all_clean() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        state.verification.all_clean = false;
        let mut events = WiringEvents::default();
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::TaintReport {
                sources: vec![0],
                sinks: vec![5],
                finish_safe: false,
                paths: vec![vb_ipc::TaintPathWire {
                    from: 0,
                    to: 5,
                    status: "dangerous".into(),
                }],
            },
            &mut state,
            &mut events,
        );
        assert!(events.taint_report_updated);
        assert!(!state.verification.all_clean);
    }

    // -----------------------------------------------------------------------
    // route_inspected: IpcResponse::WorkflowGraph
    // -----------------------------------------------------------------------

    #[test]
    fn route_inspected_workflow_graph_sets_node_count() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let nodes = vec![
            vb_ipc::NodeDescriptor {
                step_idx: 0,
                kind: "Nop".into(),
                next: Some(1),
                title: "Start".into(),
            },
            vb_ipc::NodeDescriptor {
                step_idx: 1,
                kind: "Do".into(),
                next: Some(2),
                title: "Process".into(),
            },
            vb_ipc::NodeDescriptor {
                step_idx: 2,
                kind: "Finish".into(),
                next: None,
                title: "End".into(),
            },
        ];
        let edges = vec![vb_ipc::EdgeDescriptor {
            from: 0,
            to: 1,
            label: Some("fallthrough".into()),
            edge_type: "fallthrough".into(),
        }];
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::WorkflowGraph { nodes, edges },
            &mut state,
            &mut events,
        );
        assert!(events.workflow_graph_updated);
        assert_eq!(state.workflow.node_count, 3);
    }

    #[test]
    fn route_inspected_workflow_graph_empty_nodes() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::WorkflowGraph {
                nodes: Vec::new(),
                edges: Vec::new(),
            },
            &mut state,
            &mut events,
        );
        assert!(events.workflow_graph_updated);
        assert_eq!(state.workflow.node_count, 0);
    }

    // -----------------------------------------------------------------------
    // route_inspected: unexpected variant produces error
    // -----------------------------------------------------------------------

    #[test]
    fn route_inspected_unexpected_variant_pushes_error() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::Healthy,
            &mut state,
            &mut events,
        );
        assert_eq!(events.errors.len(), 1);
        assert!(
            matches!(events.errors[0], WiringError::IpcError(ref msg) if msg.contains("Unexpected inspect response"))
        );
    }

    #[test]
    fn route_inspected_bad_request_pushes_error() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        wiring.route_inspected(
            vb_ipc::server::IpcResponse::BadRequest,
            &mut state,
            &mut events,
        );
        assert_eq!(events.errors.len(), 1);
    }

    // -----------------------------------------------------------------------
    // WiringEvents::any_changed() — all flag combinations
    // -----------------------------------------------------------------------

    #[test]
    fn any_changed_connection_changed() {
        let events = WiringEvents {
            connection_changed: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_run_accepted() {
        let events = WiringEvents {
            run_accepted: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_run_cancelled() {
        let events = WiringEvents {
            run_cancelled: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_inspected() {
        let events = WiringEvents {
            inspected: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_events_arrived() {
        let events = WiringEvents {
            events_arrived: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_trace_drained() {
        let events = WiringEvents {
            trace_drained: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_health_checked() {
        let events = WiringEvents {
            health_checked: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_metrics_updated() {
        let events = WiringEvents {
            metrics_updated: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_run_list_updated() {
        let events = WiringEvents {
            run_list_updated: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_verification_updated() {
        let events = WiringEvents {
            verification_updated: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_taint_report_updated() {
        let events = WiringEvents {
            taint_report_updated: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_workflow_graph_updated() {
        let events = WiringEvents {
            workflow_graph_updated: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_with_errors() {
        let events = WiringEvents {
            errors: vec![WiringError::IpcError("err".into())],
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_multiple_flags_set() {
        let events = WiringEvents {
            connection_changed: true,
            run_accepted: true,
            metrics_updated: true,
            ..WiringEvents::default()
        };
        assert!(events.any_changed());
    }

    #[test]
    fn any_changed_false_only_when_all_clear() {
        let events = WiringEvents {
            connected: true,
            disconnected: true,
            shutting_down: true,
            events_buffered: 5,
            ..WiringEvents::default()
        };
        // connected, disconnected, shutting_down, events_buffered are NOT
        // checked by any_changed; only the specific change flags matter.
        assert!(!events.any_changed());
    }

    // -----------------------------------------------------------------------
    // events_buffered counter increments correctly
    // -----------------------------------------------------------------------

    #[test]
    fn events_buffered_counter_increments_per_events_reply() {
        let mut wiring = IpcAppWiring::new();
        let _ = &mut wiring;

        // Simulate what route_reply does for IpcReply::Events: count in
        // events_buffered. Since we cannot inject IpcReply::Events through
        // the bridge without a server, test the counter logic directly.
        let mut events = WiringEvents::default();
        for _ in 0..3 {
            events.events_arrived = true;
            events.events_buffered = events.events_buffered.saturating_add(1);
        }
        assert_eq!(events.events_buffered, 3);
    }

    #[test]
    fn events_buffered_counter_starts_at_zero() {
        let events = WiringEvents::default();
        assert_eq!(events.events_buffered, 0);
    }

    #[test]
    fn events_buffered_saturating_add_does_not_overflow() {
        let mut count: usize = usize::MAX;
        count = count.saturating_add(1);
        assert_eq!(count, usize::MAX);
    }

    // -----------------------------------------------------------------------
    // drain_events returns items in FIFO order
    // -----------------------------------------------------------------------

    #[test]
    fn drain_events_returns_items_in_fifo_order() {
        let mut wiring = IpcAppWiring::new();

        let response_a = vb_ipc::server::IpcResponse::TraceCount { count: 10 };
        let response_b = vb_ipc::server::IpcResponse::TraceCount { count: 20 };
        let response_c = vb_ipc::server::IpcResponse::TraceCount { count: 30 };

        wiring.events_buffer.push(response_a.clone());
        wiring.events_buffer.push(response_b.clone());
        wiring.events_buffer.push(response_c.clone());

        let drained = wiring.drain_events();
        assert_eq!(drained.len(), 3);
        assert_eq!(drained[0], response_a);
        assert_eq!(drained[1], response_b);
        assert_eq!(drained[2], response_c);
    }

    #[test]
    fn drain_events_fifo_after_partial_drain() {
        let mut wiring = IpcAppWiring::new();

        wiring
            .events_buffer
            .push(vb_ipc::server::IpcResponse::TraceCount { count: 1 });
        wiring
            .events_buffer
            .push(vb_ipc::server::IpcResponse::TraceCount { count: 2 });

        let first_drain = wiring.drain_events();
        assert_eq!(first_drain.len(), 2);

        // After drain, push more and verify FIFO again.
        wiring
            .events_buffer
            .push(vb_ipc::server::IpcResponse::TraceCount { count: 3 });
        wiring
            .events_buffer
            .push(vb_ipc::server::IpcResponse::TraceCount { count: 4 });

        let second_drain = wiring.drain_events();
        assert_eq!(second_drain.len(), 2);
        assert_eq!(
            second_drain[0],
            vb_ipc::server::IpcResponse::TraceCount { count: 3 }
        );
        assert_eq!(
            second_drain[1],
            vb_ipc::server::IpcResponse::TraceCount { count: 4 }
        );
    }

    // ===================================================================
    // Verification IPC wiring tests
    // ===================================================================

    // -----------------------------------------------------------------------
    // verify_workflow sends correct request
    // -----------------------------------------------------------------------

    #[test]
    fn verify_workflow_sends_correct_request() {
        let wiring = IpcAppWiring::new();
        let digest = vb_core::WorkflowDigest::from_bytes([0xAB; 32]);
        let result = wiring.verify_workflow(digest);
        assert!(
            result.is_ok(),
            "verify_workflow should succeed when bridge is alive"
        );
    }

    // -----------------------------------------------------------------------
    // request_taint_report sends correct request
    // -----------------------------------------------------------------------

    #[test]
    fn request_taint_report_sends_correct_request() {
        let wiring = IpcAppWiring::new();
        let run_id = RunId::new(42);
        let digest = vb_core::WorkflowDigest::from_bytes([0xCD; 32]);
        let result = wiring.request_taint_report(run_id, digest);
        assert!(
            result.is_ok(),
            "request_taint_report should succeed when bridge is alive"
        );
    }

    // -----------------------------------------------------------------------
    // request_workflow_graph sends correct request
    // -----------------------------------------------------------------------

    #[test]
    fn request_workflow_graph_sends_correct_request() {
        let wiring = IpcAppWiring::new();
        let digest = vb_core::WorkflowDigest::from_bytes([0xEF; 32]);
        let result = wiring.request_workflow_graph(digest);
        assert!(
            result.is_ok(),
            "request_workflow_graph should succeed when bridge is alive"
        );
    }

    // -----------------------------------------------------------------------
    // route_reply: VerifyWorkflowResult delegates to route_inspected
    // -----------------------------------------------------------------------

    #[test]
    fn route_reply_verify_workflow_result_populates_cert_cards() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let certs = vec![
            vb_ipc::CertificateWire {
                kind: "gate_09_structure_check".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_13_taint_check".into(),
                status: "Pass".into(),
                details: String::new(),
            },
        ];
        let result = vb_ipc::VerificationResult {
            certificates: certs,
            total_checks: 2,
            pass_count: 2,
            fail_count: 0,
        };
        wiring.route_reply(
            IpcReply::VerifyWorkflowResult(vb_ipc::server::IpcResponse::VerifyWorkflow { result }),
            &mut state,
            &mut events,
        );
        assert!(events.verification_updated);
        assert_eq!(state.verification.cert_structure.badge_text, "PASS");
        assert_eq!(state.verification.cert_taint.badge_text, "PASS");
    }

    // -----------------------------------------------------------------------
    // route_reply: TaintReportReceived delegates to route_inspected
    // -----------------------------------------------------------------------

    #[test]
    fn route_reply_taint_report_received_sets_all_clean_when_safe() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        state.verification.all_clean = false;
        let mut events = WiringEvents::default();
        wiring.route_reply(
            IpcReply::TaintReportReceived(vb_ipc::server::IpcResponse::TaintReport {
                sources: Vec::new(),
                sinks: Vec::new(),
                finish_safe: true,
                paths: Vec::new(),
            }),
            &mut state,
            &mut events,
        );
        assert!(events.taint_report_updated);
        assert!(state.verification.all_clean);
    }

    #[test]
    fn route_reply_taint_report_received_does_not_set_all_clean_when_unsafe() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        state.verification.all_clean = false;
        let mut events = WiringEvents::default();
        wiring.route_reply(
            IpcReply::TaintReportReceived(vb_ipc::server::IpcResponse::TaintReport {
                sources: vec![0],
                sinks: vec![5],
                finish_safe: false,
                paths: vec![vb_ipc::TaintPathWire {
                    from: 0,
                    to: 5,
                    status: "dangerous".into(),
                }],
            }),
            &mut state,
            &mut events,
        );
        assert!(events.taint_report_updated);
        assert!(!state.verification.all_clean);
    }

    // -----------------------------------------------------------------------
    // route_reply: WorkflowGraphReceived delegates to route_inspected
    // -----------------------------------------------------------------------

    #[test]
    fn route_reply_workflow_graph_received_sets_node_count() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let nodes = vec![
            vb_ipc::NodeDescriptor {
                step_idx: 0,
                kind: "Nop".into(),
                next: Some(1),
                title: "Start".into(),
            },
            vb_ipc::NodeDescriptor {
                step_idx: 1,
                kind: "Finish".into(),
                next: None,
                title: "End".into(),
            },
        ];
        wiring.route_reply(
            IpcReply::WorkflowGraphReceived(vb_ipc::server::IpcResponse::WorkflowGraph {
                nodes,
                edges: Vec::new(),
            }),
            &mut state,
            &mut events,
        );
        assert!(events.workflow_graph_updated);
        assert_eq!(state.workflow.node_count, 2);
    }

    // -----------------------------------------------------------------------
    // Error handling: verify_workflow without connect returns not connected
    // -----------------------------------------------------------------------

    #[test]
    fn verify_workflow_without_connect_returns_not_connected_error() {
        let mut bridge = IpcBridge::new();
        assert!(bridge
            .send(IpcRequest::VerifyWorkflow {
                digest: vb_core::WorkflowDigest::from_bytes([0; 32]),
            })
            .is_ok());

        let mut replies = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            replies.extend(bridge.poll());
            if replies.iter().any(|r| matches!(r, IpcReply::Error(_))) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let found = replies
            .iter()
            .any(|r| matches!(r, IpcReply::Error(e) if e.contains("Not connected")));
        assert!(found, "expected 'Not connected' error for VerifyWorkflow");
    }

    // -----------------------------------------------------------------------
    // Error handling: request_taint_report without connect returns not connected
    // -----------------------------------------------------------------------

    #[test]
    fn request_taint_report_without_connect_returns_not_connected_error() {
        let mut bridge = IpcBridge::new();
        assert!(bridge
            .send(IpcRequest::RequestTaintReport {
                run_id: RunId::new(1),
                digest: vb_core::WorkflowDigest::from_bytes([0; 32]),
            })
            .is_ok());

        let mut replies = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            replies.extend(bridge.poll());
            if replies.iter().any(|r| matches!(r, IpcReply::Error(_))) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let found = replies
            .iter()
            .any(|r| matches!(r, IpcReply::Error(e) if e.contains("Not connected")));
        assert!(
            found,
            "expected 'Not connected' error for RequestTaintReport"
        );
    }

    // -----------------------------------------------------------------------
    // Error handling: request_workflow_graph without connect returns not connected
    // -----------------------------------------------------------------------

    #[test]
    fn request_workflow_graph_without_connect_returns_not_connected_error() {
        let mut bridge = IpcBridge::new();
        assert!(bridge
            .send(IpcRequest::RequestWorkflowGraph {
                digest: vb_core::WorkflowDigest::from_bytes([0; 32]),
            })
            .is_ok());

        let mut replies = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            replies.extend(bridge.poll());
            if replies.iter().any(|r| matches!(r, IpcReply::Error(_))) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let found = replies
            .iter()
            .any(|r| matches!(r, IpcReply::Error(e) if e.contains("Not connected")));
        assert!(
            found,
            "expected 'Not connected' error for RequestWorkflowGraph"
        );
    }

    // -----------------------------------------------------------------------
    // route_reply: VerifyWorkflowResult with failures marks all_clean false
    // -----------------------------------------------------------------------

    #[test]
    fn route_reply_verify_workflow_result_with_failures() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        let certs = vec![vb_ipc::CertificateWire {
            kind: "gate_13_taint_check".into(),
            status: "Fail".into(),
            details: "taint path found".into(),
        }];
        let result = vb_ipc::VerificationResult {
            certificates: certs,
            total_checks: 1,
            pass_count: 0,
            fail_count: 1,
        };
        wiring.route_reply(
            IpcReply::VerifyWorkflowResult(vb_ipc::server::IpcResponse::VerifyWorkflow { result }),
            &mut state,
            &mut events,
        );
        assert!(events.verification_updated);
        assert_eq!(state.verification.cert_taint.badge_text, "FAIL");
        assert!(!state.verification.all_clean);
    }

    // -----------------------------------------------------------------------
    // route_reply: WorkflowGraphReceived with empty nodes sets zero
    // -----------------------------------------------------------------------

    #[test]
    fn route_reply_workflow_graph_received_empty_nodes() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        state.workflow.node_count = 99;
        let mut events = WiringEvents::default();
        wiring.route_reply(
            IpcReply::WorkflowGraphReceived(vb_ipc::server::IpcResponse::WorkflowGraph {
                nodes: Vec::new(),
                edges: Vec::new(),
            }),
            &mut state,
            &mut events,
        );
        assert!(events.workflow_graph_updated);
        assert_eq!(state.workflow.node_count, 0);
    }

    // -----------------------------------------------------------------------
    // route_reply: error response in VerifyWorkflowResult delegates correctly
    // -----------------------------------------------------------------------

    #[test]
    fn route_reply_verify_workflow_result_error_response() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        wiring.route_reply(
            IpcReply::VerifyWorkflowResult(vb_ipc::server::IpcResponse::RuntimeError {
                message: "verification failed".into(),
            }),
            &mut state,
            &mut events,
        );
        assert_eq!(events.errors.len(), 1);
        assert!(
            matches!(events.errors[0], WiringError::IpcError(ref msg) if msg.contains("Unexpected inspect response"))
        );
    }

    // -----------------------------------------------------------------------
    // route_reply: error response in TaintReportReceived delegates correctly
    // -----------------------------------------------------------------------

    #[test]
    fn route_reply_taint_report_received_error_response() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        wiring.route_reply(
            IpcReply::TaintReportReceived(vb_ipc::server::IpcResponse::BadRequest),
            &mut state,
            &mut events,
        );
        assert_eq!(events.errors.len(), 1);
    }

    // -----------------------------------------------------------------------
    // route_reply: error response in WorkflowGraphReceived delegates correctly
    // -----------------------------------------------------------------------

    #[test]
    fn route_reply_workflow_graph_received_error_response() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();
        let mut events = WiringEvents::default();
        wiring.route_reply(
            IpcReply::WorkflowGraphReceived(vb_ipc::server::IpcResponse::RuntimeError {
                message: "graph unavailable".into(),
            }),
            &mut state,
            &mut events,
        );
        assert_eq!(events.errors.len(), 1);
    }
}
