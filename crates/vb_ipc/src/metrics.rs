#![forbid(unsafe_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Postcard serialization roundtrip tests ──

    #[test]
    fn runtime_metrics_postcard_roundtrip() {
        let metrics = RuntimeMetrics {
            shards: vec![ShardMetrics {
                shard_id: 0,
                active_runs: 5,
                ready_queue_depth: 3,
                action_queue_depth: 10,
                timer_count: 2,
                frame_pool_free: 100,
                frame_pool_total: 256,
                trace_ring_fill_pct: 45.5,
                steps_total: 1000,
                actions_total: 500,
            }],
            journal: JournalMetrics {
                writer_queue_depth: 7,
                total_events: 9999,
                total_runs: 42,
            },
            ipc: IpcMetrics {
                connected_clients: 3,
                commands_processed: 100_000,
            },
            totals: AggregateMetrics {
                runs_active: 12,
                runs_waiting: 4,
                runs_failed_total: 7,
                runs_finished_total: 200,
            },
        };
        let Ok(encoded) = postcard::to_allocvec(&metrics) else {
            return;
        };
        let decoded: RuntimeMetrics = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => {
                assert!(false, "decoding should succeed");
                return;
            }
        };
        assert_eq!(decoded, metrics);
    }

    #[test]
    fn shard_metrics_postcard_roundtrip() {
        let metrics = ShardMetrics {
            shard_id: 1,
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
        let Ok(encoded) = postcard::to_allocvec(&metrics) else {
            return;
        };
        let decoded: ShardMetrics = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => {
                assert!(false, "decoding should succeed");
                return;
            }
        };
        assert_eq!(decoded, metrics);
    }

    #[test]
    fn shard_metrics_with_max_values_roundtrip() {
        let metrics = ShardMetrics {
            shard_id: u32::MAX,
            active_runs: u32::MAX,
            ready_queue_depth: u32::MAX,
            action_queue_depth: u32::MAX,
            timer_count: u32::MAX,
            frame_pool_free: u32::MAX,
            frame_pool_total: u32::MAX,
            trace_ring_fill_pct: 100.0,
            steps_total: u64::MAX,
            actions_total: u64::MAX,
        };
        let Ok(encoded) = postcard::to_allocvec(&metrics) else {
            return;
        };
        let decoded: ShardMetrics = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => {
                assert!(false, "decoding should succeed");
                return;
            }
        };
        assert_eq!(decoded, metrics);
    }

    #[test]
    fn journal_metrics_postcard_roundtrip() {
        let metrics = JournalMetrics {
            writer_queue_depth: 42,
            total_events: u64::MAX,
            total_runs: 0,
        };
        let Ok(encoded) = postcard::to_allocvec(&metrics) else {
            return;
        };
        let decoded: JournalMetrics = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => {
                assert!(false, "decoding should succeed");
                return;
            }
        };
        assert_eq!(decoded, metrics);
    }

    #[test]
    fn ipc_metrics_postcard_roundtrip() {
        let metrics = IpcMetrics {
            connected_clients: 100,
            commands_processed: 999_999_999,
        };
        let Ok(encoded) = postcard::to_allocvec(&metrics) else {
            return;
        };
        let decoded: IpcMetrics = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => {
                assert!(false, "decoding should succeed");
                return;
            }
        };
        assert_eq!(decoded, metrics);
    }

    #[test]
    fn aggregate_metrics_postcard_roundtrip() {
        let metrics = AggregateMetrics {
            runs_active: 50,
            runs_waiting: 10,
            runs_failed_total: u64::MAX,
            runs_finished_total: u64::MAX,
        };
        let Ok(encoded) = postcard::to_allocvec(&metrics) else {
            return;
        };
        let decoded: AggregateMetrics = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => {
                assert!(false, "decoding should succeed");
                return;
            }
        };
        assert_eq!(decoded, metrics);
    }

    #[test]
    fn runtime_metrics_empty_shards_roundtrip() {
        let metrics = RuntimeMetrics {
            shards: Vec::new(),
            journal: JournalMetrics {
                writer_queue_depth: 0,
                total_events: 0,
                total_runs: 0,
            },
            ipc: IpcMetrics {
                connected_clients: 0,
                commands_processed: 0,
            },
            totals: AggregateMetrics {
                runs_active: 0,
                runs_waiting: 0,
                runs_failed_total: 0,
                runs_finished_total: 0,
            },
        };
        let Ok(encoded) = postcard::to_allocvec(&metrics) else {
            return;
        };
        let decoded: RuntimeMetrics = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => {
                assert!(false, "decoding should succeed");
                return;
            }
        };
        assert!(decoded.shards.is_empty());
        assert_eq!(decoded, metrics);
    }

    #[test]
    fn runtime_metrics_multiple_shards_roundtrip() {
        let metrics = RuntimeMetrics {
            shards: vec![
                ShardMetrics {
                    shard_id: 0,
                    active_runs: 1,
                    ready_queue_depth: 2,
                    action_queue_depth: 3,
                    timer_count: 4,
                    frame_pool_free: 5,
                    frame_pool_total: 10,
                    trace_ring_fill_pct: 50.0,
                    steps_total: 100,
                    actions_total: 50,
                },
                ShardMetrics {
                    shard_id: 1,
                    active_runs: 6,
                    ready_queue_depth: 7,
                    action_queue_depth: 8,
                    timer_count: 9,
                    frame_pool_free: 10,
                    frame_pool_total: 20,
                    trace_ring_fill_pct: 75.5,
                    steps_total: 200,
                    actions_total: 100,
                },
            ],
            journal: JournalMetrics {
                writer_queue_depth: 3,
                total_events: 300,
                total_runs: 15,
            },
            ipc: IpcMetrics {
                connected_clients: 2,
                commands_processed: 500,
            },
            totals: AggregateMetrics {
                runs_active: 7,
                runs_waiting: 3,
                runs_failed_total: 1,
                runs_finished_total: 100,
            },
        };
        let Ok(encoded) = postcard::to_allocvec(&metrics) else {
            return;
        };
        let decoded: RuntimeMetrics = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => {
                assert!(false, "decoding should succeed");
                return;
            }
        };
        assert_eq!(decoded.shards.len(), 2);
        assert_eq!(decoded, metrics);
    }

    #[test]
    fn shard_metrics_equality() {
        let a = ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        };
        let b = ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_metrics_inequality_different_shard_id() {
        let a = ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        };
        let b = ShardMetrics {
            shard_id: 1,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn aggregate_metrics_zero_values_roundtrip() {
        let metrics = AggregateMetrics {
            runs_active: 0,
            runs_waiting: 0,
            runs_failed_total: 0,
            runs_finished_total: 0,
        };
        let Ok(encoded) = postcard::to_allocvec(&metrics) else {
            return;
        };
        let decoded: AggregateMetrics = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => {
                assert!(false, "decoding should succeed");
                return;
            }
        };
        assert_eq!(decoded, metrics);
    }
}
