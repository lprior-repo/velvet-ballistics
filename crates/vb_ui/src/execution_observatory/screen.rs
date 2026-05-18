#![forbid(unsafe_code)]
use vb_ipc::RuntimeMetrics;

use crate::system::metrics::HealthStatus;
use crate::system::queue_monitor::QueueStatus;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    IpcUnavailable,
    MetricsParseError,
    RunInfoUnavailable,
    ShardNotFound,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IpcUnavailable => write!(f, "IPC bridge is disconnected"),
            Self::MetricsParseError => write!(f, "received malformed ShardMetrics from IPC"),
            Self::RunInfoUnavailable => write!(f, "per-run detail data not yet available"),
            Self::ShardNotFound => {
                write!(f, "shard_id in IPC metrics does not match known topology")
            }
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone)]
pub struct KpiRow {
    pub active_runs: KpiValue,
    pub healthy_actions: KpiValue,
    pub verification_pass_rate: KpiValue,
    pub queue_depth: KpiValue,
    pub open_incidents: KpiValue,
}

#[derive(Debug, Clone)]
pub struct KpiValue {
    pub label: String,
    pub value: f64,
    pub unit: String,
    pub trend: KpiTrend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KpiTrend {
    Up,
    Down,
    Neutral,
}

#[derive(Debug, Clone)]
pub struct ExecutionRunRow {
    pub run_id: u64,
    pub workflow_name: Option<String>,
    pub status: ExecutionStatus,
    pub started_at_ms: u64,
    pub duration_ms: Option<u64>,
    pub shard_id: u32,
    pub result: Option<ExecutionResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExecutionStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExecutionResult {
    Success,
    Failure,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct ShardFlowLane {
    pub shard_id: u32,
    pub packet_dots: Vec<PacketDot>,
    pub pressure_mark: Option<PressureMark>,
    pub action_completion_depth: u32,
    pub timer_count: u32,
}

#[derive(Debug, Clone)]
pub struct PacketDot {
    pub run_id: u64,
    pub lane_offset_ratio: f64,
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct PressureMark {
    depth: u32,
    capacity: u32,
    fill_ratio: f64,
    status: QueueStatus,
}

impl PressureMark {
    pub fn new(depth: u32, capacity: u32, fill_ratio: f64, status: QueueStatus) -> Self {
        Self {
            depth,
            capacity,
            fill_ratio: fill_ratio.clamp(0.0, 1.0),
            status,
        }
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn fill_ratio(&self) -> f64 {
        self.fill_ratio
    }

    pub fn status(&self) -> QueueStatus {
        self.status
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionEvent {
    pub seq: u64,
    pub shard: u32,
    pub run_id: Option<u64>,
    pub kind: ExecutionEventKind,
    pub summary: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExecutionEventKind {
    RunAccepted,
    StepStarted,
    ActionScheduled,
    ActionCompleted,
    RunFinished,
    RunFailed,
}

impl From<crate::system::ticker::TickerEventKind> for ExecutionEventKind {
    fn from(kind: crate::system::ticker::TickerEventKind) -> Self {
        match kind {
            crate::system::ticker::TickerEventKind::RunAccepted => Self::RunAccepted,
            crate::system::ticker::TickerEventKind::StepStarted => Self::StepStarted,
            crate::system::ticker::TickerEventKind::ActionScheduled => Self::ActionScheduled,
            crate::system::ticker::TickerEventKind::ActionCompleted => Self::ActionCompleted,
            crate::system::ticker::TickerEventKind::RunFinished => Self::RunFinished,
            crate::system::ticker::TickerEventKind::RunFailed => Self::RunFailed,
            _ => Self::RunAccepted,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemHealthCard {
    pub name: SystemHealthName,
    pub status: HealthStatus,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SystemHealthName {
    LocalServer,
    FjallStore,
    WriterQueue,
    IpcSocket,
}

pub(crate) const MAX_TICKER_EVENTS: usize = 128;
pub(crate) const MAX_EXECUTION_TABLE_ROWS: usize = 200;

pub(crate) const NOMINAL_QUEUE_CAPACITY: u32 = 256;
const PRESSURE_THRESHOLD_RATIO: f64 = 0.5;

pub struct ExecutionObservatoryScreen {
    kpis: KpiRow,
    executions_table: Vec<ExecutionRunRow>,
    shard_flow_lanes: Vec<ShardFlowLane>,
    event_ticker: Vec<ExecutionEvent>,
    system_health_cards: [SystemHealthCard; 4],
    runtime_metrics: RuntimeMetrics,
    prev_actions_total: u64,
}

impl ExecutionObservatoryScreen {
    #[must_use]
    pub fn new() -> Self {
        Self {
            kpis: KpiRow {
                active_runs: KpiValue {
                    label: String::from("Active Runs"),
                    value: 0.0,
                    unit: String::new(),
                    trend: KpiTrend::Neutral,
                },
                healthy_actions: KpiValue {
                    label: String::from("Healthy Actions"),
                    value: 0.0,
                    unit: String::from("/s"),
                    trend: KpiTrend::Neutral,
                },
                verification_pass_rate: KpiValue {
                    label: String::from("Verification Pass Rate"),
                    value: 1.0,
                    unit: String::from("%"),
                    trend: KpiTrend::Neutral,
                },
                queue_depth: KpiValue {
                    label: String::from("Queue Depth"),
                    value: 0.0,
                    unit: String::new(),
                    trend: KpiTrend::Neutral,
                },
                open_incidents: KpiValue {
                    label: String::from("Open Incidents"),
                    value: 0.0,
                    unit: String::new(),
                    trend: KpiTrend::Neutral,
                },
            },
            executions_table: Vec::with_capacity(MAX_EXECUTION_TABLE_ROWS),
            shard_flow_lanes: Vec::new(),
            event_ticker: Vec::with_capacity(MAX_TICKER_EVENTS),
            system_health_cards: [
                SystemHealthCard {
                    name: SystemHealthName::LocalServer,
                    status: HealthStatus::Healthy,
                    label: String::from("Online"),
                    detail: Some(String::from("Connected")),
                },
                SystemHealthCard {
                    name: SystemHealthName::FjallStore,
                    status: HealthStatus::Healthy,
                    label: String::from("Healthy"),
                    detail: None,
                },
                SystemHealthCard {
                    name: SystemHealthName::WriterQueue,
                    status: HealthStatus::Healthy,
                    label: String::from("Healthy"),
                    detail: Some(String::from("depth: 0")),
                },
                SystemHealthCard {
                    name: SystemHealthName::IpcSocket,
                    status: HealthStatus::Healthy,
                    label: String::from("Connected"),
                    detail: Some(String::from("3 clients")),
                },
            ],
            runtime_metrics: RuntimeMetrics {
                shards: Vec::new(),
                journal: vb_ipc::JournalMetrics {
                    writer_queue_depth: 0,
                    total_events: 0,
                    total_runs: 0,
                },
                ipc: vb_ipc::IpcMetrics {
                    connected_clients: 0,
                    commands_processed: 0,
                },
                totals: vb_ipc::AggregateMetrics {
                    runs_active: 0,
                    runs_waiting: 0,
                    runs_failed_total: 0,
                    runs_finished_total: 0,
                },
            },
            prev_actions_total: 0,
        }
    }

    pub fn update_from_runtime_metrics(&mut self, metrics: &RuntimeMetrics) -> Result<(), Error> {
        self.runtime_metrics = metrics.clone();
        self.recompute_active_runs_kpi();
        self.recompute_queue_depth_kpi();
        self.recompute_healthy_actions_kpi();
        self.recompute_verification_pass_rate_kpi();
        self.recompute_open_incidents_kpi();
        self.rebuild_shard_flow_lanes();
        self.recompute_system_health_cards();
        Ok(())
    }

    pub fn push_event(&mut self, event: ExecutionEvent) {
        if self.event_ticker.len() >= MAX_TICKER_EVENTS {
            self.event_ticker.remove(0);
        }
        self.event_ticker.push(event);
    }

    pub fn add_execution_run(&mut self, run: ExecutionRunRow) {
        self.executions_table.insert(0, run);
        self.trim_executions_table();
    }

    fn trim_executions_table(&mut self) {
        self.executions_table.truncate(MAX_EXECUTION_TABLE_ROWS);
    }

    #[must_use]
    pub fn kpis(&self) -> &KpiRow {
        &self.kpis
    }

    #[must_use]
    pub fn executions_table(&self) -> &[ExecutionRunRow] {
        &self.executions_table
    }

    #[must_use]
    pub fn shard_flow_lanes(&self) -> &[ShardFlowLane] {
        &self.shard_flow_lanes
    }

    #[must_use]
    pub fn event_ticker_events(&self) -> &[ExecutionEvent] {
        &self.event_ticker
    }

    #[must_use]
    pub fn system_health_cards(&self) -> &[SystemHealthCard; 4] {
        &self.system_health_cards
    }

    fn recompute_active_runs_kpi(&mut self) {
        let val = f64::from(self.runtime_metrics.totals.runs_active);
        self.kpis.active_runs.value = val;
    }

    fn recompute_queue_depth_kpi(&mut self) {
        let mut total: u32 = 0;
        for shard in &self.runtime_metrics.shards {
            total = total.saturating_add(shard.ready_queue_depth);
            total = total.saturating_add(shard.action_queue_depth);
        }
        self.kpis.queue_depth.value = f64::from(total);
    }

    fn recompute_healthy_actions_kpi(&mut self) {
        let mut current_total: u64 = 0;
        for shard in &self.runtime_metrics.shards {
            current_total = current_total.saturating_add(shard.actions_total);
        }
        let delta = current_total.saturating_sub(self.prev_actions_total);
        self.prev_actions_total = current_total;
        self.kpis.healthy_actions.value = f64::from(u32::try_from(delta).unwrap_or(u32::MAX));
    }

    fn recompute_verification_pass_rate_kpi(&mut self) {
        self.kpis.verification_pass_rate.value = 1.0;
    }

    fn recompute_open_incidents_kpi(&mut self) {
        self.kpis.open_incidents.value = 0.0;
    }

    fn rebuild_shard_flow_lanes(&mut self) {
        self.shard_flow_lanes.clear();
        for shard in &self.runtime_metrics.shards {
            let pressure = Self::compute_pressure_mark(
                shard
                    .ready_queue_depth
                    .saturating_add(shard.action_queue_depth),
            );
            let dots = Self::build_packet_dots(shard.shard_id, shard.active_runs);
            self.shard_flow_lanes.push(ShardFlowLane {
                shard_id: shard.shard_id,
                packet_dots: dots,
                pressure_mark: pressure,
                action_completion_depth: shard.action_queue_depth,
                timer_count: shard.timer_count,
            });
        }
        self.shard_flow_lanes.sort_by_key(|lane| lane.shard_id);
    }

    fn compute_pressure_mark(total_depth: u32) -> Option<PressureMark> {
        let fill_ratio = f64::from(total_depth) / f64::from(NOMINAL_QUEUE_CAPACITY);
        if fill_ratio < PRESSURE_THRESHOLD_RATIO {
            return None;
        }
        let status = QueueStatus::from_depth_capacity(total_depth, NOMINAL_QUEUE_CAPACITY);
        Some(PressureMark::new(
            total_depth,
            NOMINAL_QUEUE_CAPACITY,
            fill_ratio,
            status,
        ))
    }

    fn build_packet_dots(shard_id: u32, active_runs: u32) -> Vec<PacketDot> {
        if active_runs == 0 {
            return Vec::new();
        }
        let count = usize::try_from(active_runs).unwrap_or(0);
        let mut dots = Vec::with_capacity(count);
        let spacing = 1.0 / f64::from(active_runs).max(1.0);
        for i in 0..active_runs {
            let offset = (f64::from(i) + 0.5) * spacing;
            let color = if i % 2 == 0 {
                [0.0, 0.961, 1.0, 1.0]
            } else {
                [0.122, 0.478, 0.965, 1.0]
            };
            let run_id = 80_000_u64
                .saturating_add(u64::from(shard_id).saturating_mul(10_000))
                .saturating_add(u64::from(i));
            dots.push(PacketDot {
                run_id,
                lane_offset_ratio: offset,
                color,
            });
        }
        dots
    }

    fn recompute_system_health_cards(&mut self) {
        let ipc_connected = self.runtime_metrics.ipc.connected_clients > 0;
        self.system_health_cards[3] = SystemHealthCard {
            name: SystemHealthName::IpcSocket,
            status: if ipc_connected {
                HealthStatus::Healthy
            } else {
                HealthStatus::Critical
            },
            label: if ipc_connected {
                String::from("Connected")
            } else {
                String::from("Disconnected")
            },
            detail: if ipc_connected {
                Some(format!(
                    "{} clients",
                    self.runtime_metrics.ipc.connected_clients
                ))
            } else {
                Some(String::from("No IPC connection"))
            },
        };

        let writer_depth = self.runtime_metrics.journal.writer_queue_depth;
        let writer_status = QueueStatus::from_depth_capacity(writer_depth, NOMINAL_QUEUE_CAPACITY);
        self.system_health_cards[2] = SystemHealthCard {
            name: SystemHealthName::WriterQueue,
            status: match writer_status {
                QueueStatus::Normal => HealthStatus::Healthy,
                QueueStatus::Pressured => HealthStatus::Degraded,
                QueueStatus::Critical => HealthStatus::Critical,
            },
            label: match writer_status {
                QueueStatus::Normal => String::from("Healthy"),
                QueueStatus::Pressured => String::from("Pressured"),
                QueueStatus::Critical => String::from("Critical"),
            },
            detail: Some(format!("depth: {writer_depth}")),
        };

        self.system_health_cards[1] = SystemHealthCard {
            name: SystemHealthName::FjallStore,
            status: HealthStatus::Healthy,
            label: String::from("Healthy"),
            detail: None,
        };

        self.system_health_cards[0] = SystemHealthCard {
            name: SystemHealthName::LocalServer,
            status: HealthStatus::Healthy,
            label: String::from("Online"),
            detail: Some(String::from("Connected")),
        };
    }
}

impl Default for ExecutionObservatoryScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ExecutionEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RunAccepted => write!(f, "RunAccepted"),
            Self::StepStarted => write!(f, "StepStarted"),
            Self::ActionScheduled => write!(f, "ActionScheduled"),
            Self::ActionCompleted => write!(f, "ActionCompleted"),
            Self::RunFinished => write!(f, "RunFinished"),
            Self::RunFailed => write!(f, "RunFailed"),
        }
    }
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "Queued"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

impl std::fmt::Display for ExecutionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "Success"),
            Self::Failure => write!(f, "Failure"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl std::fmt::Display for SystemHealthName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalServer => write!(f, "LocalServer"),
            Self::FjallStore => write!(f, "FjallStore"),
            Self::WriterQueue => write!(f, "WriterQueue"),
            Self::IpcSocket => write!(f, "IpcSocket"),
        }
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "Healthy"),
            Self::Degraded => write!(f, "Degraded"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "Normal"),
            Self::Pressured => write!(f, "Pressured"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_ipc::ShardMetrics;

    fn stub_runtime_metrics() -> RuntimeMetrics {
        RuntimeMetrics {
            shards: vec![
                ShardMetrics {
                    shard_id: 0,
                    active_runs: 5,
                    ready_queue_depth: 20,
                    action_queue_depth: 10,
                    timer_count: 3,
                    frame_pool_free: 80,
                    frame_pool_total: 100,
                    trace_ring_fill_pct: 30.0,
                    steps_total: 100,
                    actions_total: 50,
                },
                ShardMetrics {
                    shard_id: 1,
                    active_runs: 3,
                    ready_queue_depth: 10,
                    action_queue_depth: 5,
                    timer_count: 1,
                    frame_pool_free: 90,
                    frame_pool_total: 100,
                    trace_ring_fill_pct: 20.0,
                    steps_total: 50,
                    actions_total: 25,
                },
            ],
            journal: vb_ipc::JournalMetrics {
                writer_queue_depth: 5,
                total_events: 1000,
                total_runs: 50,
            },
            ipc: vb_ipc::IpcMetrics {
                connected_clients: 3,
                commands_processed: 5000,
            },
            totals: vb_ipc::AggregateMetrics {
                runs_active: 8,
                runs_waiting: 2,
                runs_failed_total: 1,
                runs_finished_total: 49,
            },
        }
    }

    #[test]
    fn new_screen_has_placeholder_kpis() {
        let screen = ExecutionObservatoryScreen::new();
        assert_eq!(screen.kpis.active_runs.value, 0.0);
        assert_eq!(screen.kpis.verification_pass_rate.value, 1.0);
        assert_eq!(screen.kpis.queue_depth.value, 0.0);
    }

    #[test]
    fn new_screen_has_empty_executions_table() {
        let screen = ExecutionObservatoryScreen::new();
        assert!(screen.executions_table.is_empty());
    }

    #[test]
    fn new_screen_has_empty_shard_flow_lanes() {
        let screen = ExecutionObservatoryScreen::new();
        assert!(screen.shard_flow_lanes.is_empty());
    }

    #[test]
    fn new_screen_has_empty_event_ticker() {
        let screen = ExecutionObservatoryScreen::new();
        assert!(screen.event_ticker.is_empty());
    }

    #[test]
    fn new_screen_has_four_health_cards() {
        let screen = ExecutionObservatoryScreen::new();
        let cards = screen.system_health_cards();
        assert_eq!(cards.len(), 4);
        assert_eq!(cards[0].name, SystemHealthName::LocalServer);
        assert_eq!(cards[1].name, SystemHealthName::FjallStore);
        assert_eq!(cards[2].name, SystemHealthName::WriterQueue);
        assert_eq!(cards[3].name, SystemHealthName::IpcSocket);
    }

    #[test]
    fn update_from_runtime_metrics_sets_active_runs() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = stub_runtime_metrics();
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        assert_eq!(screen.kpis.active_runs.value, 8.0);
    }

    #[test]
    fn update_from_runtime_metrics_sets_queue_depth() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = stub_runtime_metrics();
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        assert_eq!(screen.kpis.queue_depth.value, 45.0);
    }

    #[test]
    fn update_from_runtime_metrics_rebuilds_shard_flow_lanes() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = stub_runtime_metrics();
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        assert_eq!(screen.shard_flow_lanes.len(), 2);
        assert_eq!(
            screen.shard_flow_lanes.first().map(|lane| lane.shard_id),
            Some(0)
        );
        assert_eq!(
            screen.shard_flow_lanes.get(1).map(|lane| lane.shard_id),
            Some(1)
        );
    }

    #[test]
    fn shard_flow_lanes_sorted_by_shard_id() {
        let mut screen = ExecutionObservatoryScreen::new();
        let mut metrics = stub_runtime_metrics();
        metrics.shards.reverse();
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        assert_eq!(
            screen.shard_flow_lanes.first().map(|lane| lane.shard_id),
            Some(0)
        );
        assert_eq!(
            screen.shard_flow_lanes.get(1).map(|lane| lane.shard_id),
            Some(1)
        );
    }

    #[test]
    fn pressure_mark_none_when_below_threshold() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = RuntimeMetrics {
            shards: vec![ShardMetrics {
                shard_id: 0,
                active_runs: 1,
                ready_queue_depth: 10,
                action_queue_depth: 5,
                timer_count: 0,
                frame_pool_free: 100,
                frame_pool_total: 100,
                trace_ring_fill_pct: 10.0,
                steps_total: 0,
                actions_total: 0,
            }],
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
                runs_active: 1,
                runs_waiting: 0,
                runs_failed_total: 0,
                runs_finished_total: 0,
            },
        };
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        assert_eq!(
            screen
                .shard_flow_lanes
                .first()
                .map(|lane| lane.pressure_mark.is_none()),
            Some(true)
        );
    }

    #[test]
    fn pressure_mark_some_when_above_threshold() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = RuntimeMetrics {
            shards: vec![ShardMetrics {
                shard_id: 0,
                active_runs: 1,
                ready_queue_depth: 150,
                action_queue_depth: 50,
                timer_count: 0,
                frame_pool_free: 100,
                frame_pool_total: 100,
                trace_ring_fill_pct: 10.0,
                steps_total: 0,
                actions_total: 0,
            }],
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
                runs_active: 1,
                runs_waiting: 0,
                runs_failed_total: 0,
                runs_finished_total: 0,
            },
        };
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        let pressure = screen
            .shard_flow_lanes
            .first()
            .and_then(|lane| lane.pressure_mark.as_ref());
        assert_eq!(pressure.map(PressureMark::depth), Some(200));
        assert_eq!(pressure.map(PressureMark::capacity), Some(256));
        assert_eq!(
            pressure.map(|pm| (pm.fill_ratio() - 0.78125).abs() < 0.001),
            Some(true)
        );
    }

    #[test]
    fn packet_dots_empty_when_no_active_runs() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = RuntimeMetrics {
            shards: vec![ShardMetrics {
                shard_id: 0,
                active_runs: 0,
                ready_queue_depth: 0,
                action_queue_depth: 0,
                timer_count: 0,
                frame_pool_free: 100,
                frame_pool_total: 100,
                trace_ring_fill_pct: 0.0,
                steps_total: 0,
                actions_total: 0,
            }],
            journal: vb_ipc::JournalMetrics {
                writer_queue_depth: 0,
                total_events: 0,
                total_runs: 0,
            },
            ipc: vb_ipc::IpcMetrics {
                connected_clients: 0,
                commands_processed: 0,
            },
            totals: vb_ipc::AggregateMetrics {
                runs_active: 0,
                runs_waiting: 0,
                runs_failed_total: 0,
                runs_finished_total: 0,
            },
        };
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        assert_eq!(
            screen
                .shard_flow_lanes
                .first()
                .map(|lane| lane.packet_dots.is_empty()),
            Some(true)
        );
    }

    #[test]
    fn packet_dots_match_active_runs_count() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = RuntimeMetrics {
            shards: vec![ShardMetrics {
                shard_id: 0,
                active_runs: 5,
                ready_queue_depth: 0,
                action_queue_depth: 0,
                timer_count: 0,
                frame_pool_free: 100,
                frame_pool_total: 100,
                trace_ring_fill_pct: 0.0,
                steps_total: 0,
                actions_total: 0,
            }],
            journal: vb_ipc::JournalMetrics {
                writer_queue_depth: 0,
                total_events: 0,
                total_runs: 0,
            },
            ipc: vb_ipc::IpcMetrics {
                connected_clients: 0,
                commands_processed: 0,
            },
            totals: vb_ipc::AggregateMetrics {
                runs_active: 5,
                runs_waiting: 0,
                runs_failed_total: 0,
                runs_finished_total: 0,
            },
        };
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        assert_eq!(
            screen
                .shard_flow_lanes
                .first()
                .map(|lane| lane.packet_dots.len()),
            Some(5)
        );
    }

    #[test]
    fn packet_dots_have_offset_ratio_in_valid_range() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = RuntimeMetrics {
            shards: vec![ShardMetrics {
                shard_id: 0,
                active_runs: 4,
                ready_queue_depth: 0,
                action_queue_depth: 0,
                timer_count: 0,
                frame_pool_free: 100,
                frame_pool_total: 100,
                trace_ring_fill_pct: 0.0,
                steps_total: 0,
                actions_total: 0,
            }],
            journal: vb_ipc::JournalMetrics {
                writer_queue_depth: 0,
                total_events: 0,
                total_runs: 0,
            },
            ipc: vb_ipc::IpcMetrics {
                connected_clients: 0,
                commands_processed: 0,
            },
            totals: vb_ipc::AggregateMetrics {
                runs_active: 4,
                runs_waiting: 0,
                runs_failed_total: 0,
                runs_finished_total: 0,
            },
        };
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        if let Some(lane) = screen.shard_flow_lanes.first() {
            for dot in &lane.packet_dots {
                assert!(dot.lane_offset_ratio >= 0.0);
                assert!(dot.lane_offset_ratio <= 1.0);
            }
        }
    }

    #[test]
    fn push_event_append_to_ticker() {
        let mut screen = ExecutionObservatoryScreen::new();
        let event = ExecutionEvent {
            seq: 1,
            shard: 0,
            run_id: Some(100),
            kind: ExecutionEventKind::RunAccepted,
            summary: String::from("Run #100 accepted"),
            timestamp_ms: 1_000_000,
        };
        screen.push_event(event);
        assert_eq!(screen.event_ticker.len(), 1);
        assert_eq!(screen.event_ticker.first().map(|event| event.seq), Some(1));
    }

    #[test]
    fn push_event_evicts_oldest_at_capacity() {
        let mut screen = ExecutionObservatoryScreen::new();
        for i in 0..(MAX_TICKER_EVENTS + 5) {
            let event = ExecutionEvent {
                seq: u64::try_from(i).unwrap_or(u64::MAX),
                shard: 0,
                run_id: None,
                kind: ExecutionEventKind::RunAccepted,
                summary: format!("event-{i}"),
                timestamp_ms: u64::try_from(i).unwrap_or(u64::MAX),
            };
            screen.push_event(event);
        }
        assert_eq!(screen.event_ticker.len(), MAX_TICKER_EVENTS);
        assert_eq!(screen.event_ticker.first().map(|event| event.seq), Some(5));
    }

    #[test]
    fn add_execution_run_inserts_newest_first() {
        let mut screen = ExecutionObservatoryScreen::new();
        let run1 = ExecutionRunRow {
            run_id: 1,
            workflow_name: Some(String::from("wf-a")),
            status: ExecutionStatus::Completed,
            started_at_ms: 1_000_000,
            duration_ms: Some(100),
            shard_id: 0,
            result: Some(ExecutionResult::Success),
        };
        let run2 = ExecutionRunRow {
            run_id: 2,
            workflow_name: Some(String::from("wf-b")),
            status: ExecutionStatus::Running,
            started_at_ms: 1_001_000,
            duration_ms: None,
            shard_id: 1,
            result: None,
        };
        screen.add_execution_run(run1);
        screen.add_execution_run(run2);
        assert_eq!(screen.executions_table.len(), 2);
        assert_eq!(
            screen.executions_table.first().map(|run| run.run_id),
            Some(2)
        );
        assert_eq!(
            screen.executions_table.get(1).map(|run| run.run_id),
            Some(1)
        );
    }

    #[test]
    fn add_execution_run_trims_at_max_rows() {
        let mut screen = ExecutionObservatoryScreen::new();
        for i in 0..(MAX_EXECUTION_TABLE_ROWS + 50) {
            let run = ExecutionRunRow {
                run_id: u64::try_from(i).unwrap_or(u64::MAX),
                workflow_name: Some(format!("wf-{i}")),
                status: ExecutionStatus::Completed,
                started_at_ms: u64::try_from(i).unwrap_or(u64::MAX),
                duration_ms: Some(100),
                shard_id: 0,
                result: Some(ExecutionResult::Success),
            };
            screen.add_execution_run(run);
        }
        assert_eq!(screen.executions_table.len(), MAX_EXECUTION_TABLE_ROWS);
        assert_eq!(
            screen.executions_table.first().map(|run| run.run_id),
            u64::try_from(MAX_EXECUTION_TABLE_ROWS)
                .ok()
                .and_then(|rows| rows.checked_add(49))
        );
    }

    #[test]
    fn health_cards_update_ipc_status_when_disconnected() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = RuntimeMetrics {
            shards: Vec::new(),
            journal: vb_ipc::JournalMetrics {
                writer_queue_depth: 0,
                total_events: 0,
                total_runs: 0,
            },
            ipc: vb_ipc::IpcMetrics {
                connected_clients: 0,
                commands_processed: 0,
            },
            totals: vb_ipc::AggregateMetrics {
                runs_active: 0,
                runs_waiting: 0,
                runs_failed_total: 0,
                runs_finished_total: 0,
            },
        };
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        let cards = screen.system_health_cards();
        assert_eq!(cards[3].status, HealthStatus::Critical);
        assert_eq!(cards[3].label, "Disconnected");
    }

    #[test]
    fn health_cards_update_writer_queue_pressure() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = RuntimeMetrics {
            shards: Vec::new(),
            journal: vb_ipc::JournalMetrics {
                writer_queue_depth: 200,
                total_events: 0,
                total_runs: 0,
            },
            ipc: vb_ipc::IpcMetrics {
                connected_clients: 1,
                commands_processed: 0,
            },
            totals: vb_ipc::AggregateMetrics {
                runs_active: 0,
                runs_waiting: 0,
                runs_failed_total: 0,
                runs_finished_total: 0,
            },
        };
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        let cards = screen.system_health_cards();
        assert_eq!(cards[2].name, SystemHealthName::WriterQueue);
        assert!(cards[2].detail.is_some());
    }

    #[test]
    fn verification_pass_rate_clamped_to_one() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = stub_runtime_metrics();
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        assert!(screen.kpis.verification_pass_rate.value >= 0.0);
        assert!(screen.kpis.verification_pass_rate.value <= 1.0);
    }

    #[test]
    fn open_incidents_default_to_zero() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = stub_runtime_metrics();
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        assert_eq!(screen.kpis.open_incidents.value, 0.0);
    }

    #[test]
    fn accessors_return_empty_not_none() {
        let screen = ExecutionObservatoryScreen::new();
        assert!(screen.kpis().active_runs.value >= 0.0);
        assert!(screen.executions_table().is_empty());
        assert!(screen.shard_flow_lanes().is_empty());
        assert!(screen.event_ticker_events().is_empty());
    }

    #[test]
    fn execution_event_kind_display() {
        assert_eq!(
            format!("{}", ExecutionEventKind::RunAccepted),
            "RunAccepted"
        );
        assert_eq!(format!("{}", ExecutionEventKind::RunFailed), "RunFailed");
        assert_eq!(
            format!("{}", ExecutionEventKind::StepStarted),
            "StepStarted"
        );
    }

    #[test]
    fn execution_status_display() {
        assert_eq!(format!("{}", ExecutionStatus::Queued), "Queued");
        assert_eq!(format!("{}", ExecutionStatus::Running), "Running");
        assert_eq!(format!("{}", ExecutionStatus::Completed), "Completed");
        assert_eq!(format!("{}", ExecutionStatus::Failed), "Failed");
    }

    #[test]
    fn execution_result_display() {
        assert_eq!(format!("{}", ExecutionResult::Success), "Success");
        assert_eq!(format!("{}", ExecutionResult::Failure), "Failure");
        assert_eq!(format!("{}", ExecutionResult::Cancelled), "Cancelled");
    }

    #[test]
    fn system_health_name_display() {
        assert_eq!(format!("{}", SystemHealthName::LocalServer), "LocalServer");
        assert_eq!(format!("{}", SystemHealthName::FjallStore), "FjallStore");
        assert_eq!(format!("{}", SystemHealthName::WriterQueue), "WriterQueue");
        assert_eq!(format!("{}", SystemHealthName::IpcSocket), "IpcSocket");
    }

    #[test]
    fn health_status_display() {
        assert_eq!(format!("{}", HealthStatus::Healthy), "Healthy");
        assert_eq!(format!("{}", HealthStatus::Degraded), "Degraded");
        assert_eq!(format!("{}", HealthStatus::Critical), "Critical");
    }

    #[test]
    fn queue_status_display() {
        assert_eq!(format!("{}", QueueStatus::Normal), "Normal");
        assert_eq!(format!("{}", QueueStatus::Pressured), "Pressured");
        assert_eq!(format!("{}", QueueStatus::Critical), "Critical");
    }

    #[test]
    fn error_display() {
        assert_eq!(
            format!("{}", Error::IpcUnavailable),
            "IPC bridge is disconnected"
        );
        assert_eq!(
            format!("{}", Error::MetricsParseError),
            "received malformed ShardMetrics from IPC"
        );
        assert_eq!(
            format!("{}", Error::RunInfoUnavailable),
            "per-run detail data not yet available"
        );
        assert_eq!(
            format!("{}", Error::ShardNotFound),
            "shard_id in IPC metrics does not match known topology"
        );
    }

    #[test]
    fn error_debug() {
        let err = Error::IpcUnavailable;
        let debug = format!("{:?}", err);
        assert!(debug.contains("IpcUnavailable"));
    }

    #[test]
    fn ticker_event_kind_from_ticker_event_kind() {
        use crate::system::ticker::TickerEventKind;
        assert_eq!(
            ExecutionEventKind::from(TickerEventKind::RunAccepted),
            ExecutionEventKind::RunAccepted
        );
        assert_eq!(
            ExecutionEventKind::from(TickerEventKind::RunFailed),
            ExecutionEventKind::RunFailed
        );
        assert_eq!(
            ExecutionEventKind::from(TickerEventKind::StepStarted),
            ExecutionEventKind::StepStarted
        );
        assert_eq!(
            ExecutionEventKind::from(TickerEventKind::ActionScheduled),
            ExecutionEventKind::ActionScheduled
        );
        assert_eq!(
            ExecutionEventKind::from(TickerEventKind::ActionCompleted),
            ExecutionEventKind::ActionCompleted
        );
        assert_eq!(
            ExecutionEventKind::from(TickerEventKind::RunFinished),
            ExecutionEventKind::RunFinished
        );
        assert_eq!(
            ExecutionEventKind::from(TickerEventKind::StepSucceeded),
            ExecutionEventKind::RunAccepted
        );
        assert_eq!(
            ExecutionEventKind::from(TickerEventKind::ActionFailed),
            ExecutionEventKind::RunAccepted
        );
        assert_eq!(
            ExecutionEventKind::from(TickerEventKind::Other),
            ExecutionEventKind::RunAccepted
        );
    }

    #[test]
    fn kpi_value_fields() {
        let kv = KpiValue {
            label: String::from("Test KPI"),
            value: 42.5,
            unit: String::from("ms"),
            trend: KpiTrend::Up,
        };
        assert_eq!(kv.label, "Test KPI");
        assert!((kv.value - 42.5).abs() < f64::EPSILON);
        assert_eq!(kv.unit, "ms");
        assert_eq!(kv.trend, KpiTrend::Up);
    }

    #[test]
    fn kpi_row_all_five_present() {
        let screen = ExecutionObservatoryScreen::new();
        let _ = &screen.kpis.active_runs;
        let _ = &screen.kpis.healthy_actions;
        let _ = &screen.kpis.verification_pass_rate;
        let _ = &screen.kpis.queue_depth;
        let _ = &screen.kpis.open_incidents;
    }

    #[test]
    fn execution_run_row_duration_none_for_running() {
        let run = ExecutionRunRow {
            run_id: 1,
            workflow_name: Some(String::from("wf")),
            status: ExecutionStatus::Running,
            started_at_ms: 1_000_000,
            duration_ms: None,
            shard_id: 0,
            result: None,
        };
        assert!(run.duration_ms.is_none());
        assert!(run.result.is_none());
    }

    #[test]
    fn execution_run_row_has_result_for_completed() {
        let run = ExecutionRunRow {
            run_id: 1,
            workflow_name: Some(String::from("wf")),
            status: ExecutionStatus::Completed,
            started_at_ms: 1_000_000,
            duration_ms: Some(500),
            shard_id: 0,
            result: Some(ExecutionResult::Success),
        };
        assert!(run.duration_ms.is_some());
        assert!(run.result.is_some());
    }

    #[test]
    fn default_equals_new() {
        let from_new = ExecutionObservatoryScreen::new();
        let from_default = ExecutionObservatoryScreen::default();
        assert_eq!(
            from_new.kpis.active_runs.value,
            from_default.kpis.active_runs.value
        );
        assert_eq!(
            from_new.executions_table.len(),
            from_default.executions_table.len()
        );
        assert_eq!(
            from_new.shard_flow_lanes.len(),
            from_default.shard_flow_lanes.len()
        );
    }

    #[test]
    fn pressure_mark_fill_ratio_clamped_to_one() {
        let pm = PressureMark::new(300, 256, 1.2, QueueStatus::Critical);
        assert!((pm.fill_ratio() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn shard_flow_lane_timer_count_matches_metrics() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = RuntimeMetrics {
            shards: vec![ShardMetrics {
                shard_id: 3,
                active_runs: 2,
                ready_queue_depth: 0,
                action_queue_depth: 0,
                timer_count: 42,
                frame_pool_free: 100,
                frame_pool_total: 100,
                trace_ring_fill_pct: 0.0,
                steps_total: 0,
                actions_total: 0,
            }],
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
                runs_active: 2,
                runs_waiting: 0,
                runs_failed_total: 0,
                runs_finished_total: 0,
            },
        };
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        assert_eq!(
            screen.shard_flow_lanes.first().map(|lane| lane.timer_count),
            Some(42)
        );
    }

    #[test]
    fn shard_flow_lane_action_completion_depth_matches_metrics() {
        let mut screen = ExecutionObservatoryScreen::new();
        let metrics = RuntimeMetrics {
            shards: vec![ShardMetrics {
                shard_id: 0,
                active_runs: 1,
                ready_queue_depth: 0,
                action_queue_depth: 17,
                timer_count: 0,
                frame_pool_free: 100,
                frame_pool_total: 100,
                trace_ring_fill_pct: 0.0,
                steps_total: 0,
                actions_total: 0,
            }],
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
                runs_active: 1,
                runs_waiting: 0,
                runs_failed_total: 0,
                runs_finished_total: 0,
            },
        };
        assert!(screen.update_from_runtime_metrics(&metrics).is_ok());
        assert_eq!(
            screen
                .shard_flow_lanes
                .first()
                .map(|lane| lane.action_completion_depth),
            Some(17)
        );
    }
}
