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
