//! IPC metrics types.

use serde::{Deserialize, Serialize};

/// Runtime metrics response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeMetrics {
    /// Per-shard metrics.
    pub shards: Vec<ShardMetrics>,
    /// Journal metrics.
    pub journal: JournalMetrics,
    /// IPC connection metrics.
    pub ipc: IpcMetrics,
    /// Aggregate totals across all shards.
    pub totals: AggregateMetrics,
}

/// Per-shard metrics snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShardMetrics {
    /// Shard index.
    pub shard_id: u32,
    /// Number of active runs on this shard.
    pub active_runs: u32,
    /// Number of commands waiting in the ready queue.
    pub ready_queue_depth: u32,
    /// Remaining capacity in the command queue.
    pub action_queue_depth: u32,
    /// Number of pending timers.
    pub timer_count: u32,
    /// Free frames in the frame pool.
    pub frame_pool_free: u32,
    /// Total capacity of the frame pool.
    pub frame_pool_total: u32,
    /// Trace ring fill percentage (0.0 - 100.0).
    pub trace_ring_fill_pct: f32,
    /// Total steps executed on this shard.
    pub steps_total: u64,
    /// Total actions completed on this shard.
    pub actions_total: u64,
}

/// Journal metrics snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalMetrics {
    /// Journal writer queue depth.
    pub writer_queue_depth: u32,
    /// Total events written.
    pub total_events: u64,
    /// Total runs recorded.
    pub total_runs: u64,
}

/// IPC connection metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpcMetrics {
    /// Currently connected IPC clients.
    pub connected_clients: u32,
    /// Total IPC commands processed.
    pub commands_processed: u64,
}

/// Aggregate totals across all shards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregateMetrics {
    /// Total runs currently active across all shards.
    pub runs_active: u32,
    /// Total runs waiting (suspended on actions or timers).
    pub runs_waiting: u32,
    /// Total runs failed since startup.
    pub runs_failed_total: u64,
    /// Total runs finished since startup.
    pub runs_finished_total: u64,
}
