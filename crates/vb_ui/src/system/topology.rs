use super::metrics::ShardDisplay;

#[derive(Debug, Clone)]
pub struct TopologySnapshot {
    pub shards: Vec<ShardDisplay>,
    pub journal_writer_status: JournalStatus,
    pub ipc_connections: u32,
}

#[derive(Debug, Clone)]
pub struct JournalStatus {
    pub queue_depth: u32,
    pub avg_latency_us: u64,
    pub healthy: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::metrics::HealthStatus;
    use std::time::Duration;

    fn stub_shard(id: u32) -> ShardDisplay {
        ShardDisplay {
            shard_id: id,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_per_sec: 0.0,
            tick_duration_p95: Duration::ZERO,
            health: HealthStatus::Healthy,
        }
    }

    #[test]
    fn topology_snapshot_holds_shards_and_journal() {
        let snap = TopologySnapshot {
            shards: vec![stub_shard(0), stub_shard(1)],
            journal_writer_status: JournalStatus {
                queue_depth: 3,
                avg_latency_us: 150,
                healthy: true,
            },
            ipc_connections: 2,
        };
        assert_eq!(snap.shards.len(), 2);
        assert!(snap.journal_writer_status.healthy);
        assert_eq!(snap.ipc_connections, 2);
    }

    #[test]
    fn journal_status_unhealthy() {
        let status = JournalStatus {
            queue_depth: 500,
            avg_latency_us: 10_000,
            healthy: false,
        };
        assert!(!status.healthy);
    }
}
