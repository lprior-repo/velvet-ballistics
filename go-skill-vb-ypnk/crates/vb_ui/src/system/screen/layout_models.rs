#![forbid(unsafe_code)]
//! System screen layout models and color constants.

use crate::system::queue_monitor::QueueStatus;

pub const SYS_PANEL_BG: &str = "#12121f";
pub const SYS_CARD_BG: &str = "#16162a";
pub const SYS_BORDER: &str = "#2a2a4a";
pub const SYS_TEXT_PRIMARY: &str = "#e8e8ff";
pub const SYS_TEXT_SECONDARY: &str = "#8888aa";
pub const SYS_NEON_CYAN: &str = "#00f5ff";
pub const SYS_NEON_GREEN: &str = "#39ff14";
pub const SYS_NEON_RED: &str = "#ff073a";
pub const SYS_NEON_ORANGE: &str = "#ff6b00";
pub const SYS_NEON_YELLOW: &str = "#ffe600";
pub const SYS_NEON_PURPLE: &str = "#b14dff";
pub const SYS_TEXT_DIM: &str = "#555577";
pub const SYS_CANVAS_BG: &str = "#0a0a12";

#[derive(Debug, Clone)]
pub struct TopologyShardRow {
    pub shard_id: u32,
    pub status_label: String,
    pub status_color: String,
    pub active_runs: u32,
    pub bg_color: String,
}

#[derive(Debug, Clone)]
pub struct JournalStatusRow {
    pub label: String,
    pub label_color: String,
    pub queue_depth: u32,
}

#[derive(Debug, Clone)]
pub struct TopologyPanel {
    pub shard_rows: Vec<TopologyShardRow>,
    pub journal_status: JournalStatusRow,
    pub timer_count: u32,
    pub ipc_connections: u32,
}

#[derive(Debug, Clone)]
pub struct ActivitySegment {
    pub run_id: u64,
    pub width_ratio: f64,
    pub color: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct ActivityLane {
    pub shard_id: u32,
    pub active_runs: u32,
    pub ready_queue_depth: u32,
    pub action_queue_depth: u32,
    pub steps_per_sec: f64,
    pub segments: Vec<ActivitySegment>,
    pub lane_label_color: String,
}

#[derive(Debug, Clone)]
pub struct QueueMonitorBar {
    pub label: String,
    pub fill_color: String,
    pub fill_ratio: f64,
    pub depth_text: String,
    pub status: QueueStatus,
}

#[derive(Debug, Clone)]
pub struct QueueMonitorPanel {
    pub bars: Vec<QueueMonitorBar>,
}

#[derive(Debug, Clone)]
pub struct TickerChip {
    pub kind_label: String,
    pub bg_color: String,
    pub text_color: String,
    pub summary: String,
    pub seq: u64,
}

#[derive(Debug, Clone)]
pub struct EventTickerPanel {
    pub chips: Vec<TickerChip>,
}

#[derive(Debug, Clone)]
pub struct AlertCard {
    pub severity_label: String,
    pub severity_color: String,
    pub message: String,
    pub source: String,
    pub bg_color: String,
    pub acknowledged: bool,
}

#[derive(Debug, Clone)]
pub struct AlertStack {
    pub alerts: Vec<AlertCard>,
}

#[derive(Debug, Clone)]
pub struct LatencySegment {
    pub label: String,
    pub avg_us: u64,
    pub display: String,
    pub fill_color: String,
    pub width_ratio: f64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
}

#[derive(Debug, Clone)]
pub struct LatencyBreakdown {
    pub segments: Vec<LatencySegment>,
}
