#![forbid(unsafe_code)]
//! System topology data layer.
//!
//! `TopologySnapshot` wraps a `SystemTopology` (from map.rs) and the
//! per-shard display metrics, plus journal/IPC metadata. It computes
//! aggregate health from the raw shard topology and provides convenience
//! methods used by `SystemScreen`.

use crate::system::map::{ShardNode, ShardStatus, SystemTopology};
use crate::system::metrics::{HealthStatus, ShardDisplay, SystemMetrics};

// ---------------------------------------------------------------------------
// JournalStatus
// ---------------------------------------------------------------------------

/// Status of the journal writer subsystem.
#[derive(Debug, Clone)]
pub struct JournalStatus {
    pub queue_depth: u32,
    pub avg_latency_us: u64,
    pub healthy: bool,
}

// ---------------------------------------------------------------------------
// TopologySnapshot
// ---------------------------------------------------------------------------

/// Aggregated snapshot of the system topology plus derived metrics.
///
/// Construct via `TopologySnapshot::from_shards(Vec<ShardNode>)` to build
/// everything from a flat list of `ShardNode`s. The struct also carries
/// per-shard display data for the UI layer and journal/IPC metadata.
#[derive(Debug, Clone)]
pub struct TopologySnapshot {
    /// Per-shard display rows used by the rendering layer.
    pub shards: Vec<ShardDisplay>,
    /// Journal writer subsystem status.
    pub journal_writer_status: JournalStatus,
    /// Number of active IPC connections.
    pub ipc_connections: u32,
    /// The raw topology from map.rs.
    pub topology: SystemTopology,
    /// Worst shard status across all shards.
    pub worst_status: ShardStatus,
    /// Total active runs summed across shards.
    pub total_active_runs: u32,
    /// Total pending actions (ready + action queue depths) across shards.
    pub total_pending: u32,
    /// Derived system-wide metrics.
    pub metrics: SystemMetrics,
}

impl TopologySnapshot {
    /// Build a fully-populated `TopologySnapshot` from a flat list of `ShardNode`s.
    ///
    /// This constructs the inner `SystemTopology`, derives metrics,
    /// and sets journal/IPC to default (healthy, zero) values.
    #[must_use]
    pub fn from_shards(shards: Vec<ShardNode>) -> Self {
        let topology = SystemTopology {
            shards: shards.clone(),
        };
        let worst_status = topology.worst_status();
        let total_active_runs = topology.total_active_runs();
        let total_pending = topology.total_pending_actions();
        let metrics = Self::derive_metrics(&topology);

        // Map ShardNode -> ShardDisplay for the display layer.
        let display_shards = Self::shard_displays(&topology);

        Self {
            shards: display_shards,
            journal_writer_status: JournalStatus {
                queue_depth: 0,
                avg_latency_us: 0,
                healthy: true,
            },
            ipc_connections: 0,
            topology,
            worst_status,
            total_active_runs,
            total_pending,
            metrics,
        }
    }

    /// Returns `true` when the worst shard status is `Idle` or `Active`
    /// (i.e., no shard is `Overloaded`).
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        matches!(self.worst_status, ShardStatus::Idle | ShardStatus::Active)
    }

    /// Returns a formatted single-line status summary string.
    ///
    /// Format: `"[Idle|Active|Overloaded] shards=N active=Runs pending=Pending"`
    #[must_use]
    pub fn summary_text(&self) -> String {
        let status_label = match self.worst_status {
            ShardStatus::Idle => "Idle",
            ShardStatus::Active => "Active",
            ShardStatus::Overloaded => "Overloaded",
        };
        let shard_count = self.topology.shards.len();
        format!(
            "{} shards={} active={} pending={}",
            status_label, shard_count, self.total_active_runs, self.total_pending
        )
    }

    // -- Internal helpers ----------------------------------------------------

    /// Derive `SystemMetrics` from the topology's shard nodes.
    fn derive_metrics(topology: &SystemTopology) -> SystemMetrics {
        let mut total_active_runs = 0u32;
        let mut total_ready_queue_depth = 0u32;
        let mut total_action_queue_depth = 0u32;
        let mut any_critical = false;
        let mut any_degraded = false;

        let shard_displays: Vec<ShardDisplay> = topology
            .shards
            .iter()
            .map(|node| {
                let health = Self::health_for_node(node);
                if health == HealthStatus::Critical {
                    any_critical = true;
                } else if health == HealthStatus::Degraded {
                    any_degraded = true;
                }

                total_active_runs = total_active_runs.saturating_add(node.active_runs);
                total_ready_queue_depth = total_ready_queue_depth.saturating_add(node.ready_depth);
                total_action_queue_depth =
                    total_action_queue_depth.saturating_add(node.action_depth);

                ShardDisplay {
                    shard_id: node.shard_id,
                    active_runs: node.active_runs,
                    ready_queue_depth: node.ready_depth,
                    action_queue_depth: node.action_depth,
                    timer_count: 0,
                    frame_pool_free: 0,
                    frame_pool_total: 0,
                    trace_ring_fill_pct: 0.0,
                    steps_per_sec: 0.0,
                    tick_duration_p95: std::time::Duration::ZERO,
                    health,
                }
            })
            .collect();

        let overall_health = if any_critical {
            HealthStatus::Critical
        } else if any_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        SystemMetrics {
            shards: shard_displays,
            total_active_runs,
            total_ready_queue_depth,
            total_action_queue_depth,
            overall_health,
        }
    }

    /// Build `Vec<ShardDisplay>` from topology nodes.
    fn shard_displays(topology: &SystemTopology) -> Vec<ShardDisplay> {
        topology
            .shards
            .iter()
            .map(|node| ShardDisplay {
                shard_id: node.shard_id,
                active_runs: node.active_runs,
                ready_queue_depth: node.ready_depth,
                action_queue_depth: node.action_depth,
                timer_count: 0,
                frame_pool_free: 0,
                frame_pool_total: 0,
                trace_ring_fill_pct: 0.0,
                steps_per_sec: 0.0,
                tick_duration_p95: std::time::Duration::ZERO,
                health: Self::health_for_node(node),
            })
            .collect()
    }

    /// Determine the health of a shard node based on its status and load.
    ///
    /// An `Overloaded` shard is always `Critical`. An `Active` shard whose
    /// queues are above 80% of capacity is `Critical`, above 50% is `Degraded`.
    /// Idle shards are `Healthy`.
    fn health_for_node(node: &ShardNode) -> HealthStatus {
        if node.status == ShardStatus::Overloaded {
            return HealthStatus::Critical;
        }
        if node.status == ShardStatus::Idle {
            return HealthStatus::Healthy;
        }
        // Active shard: check queue pressure relative to max_runs as proxy capacity.
        let pending = node.ready_depth.saturating_add(node.action_depth);
        let capacity = node.max_runs;
        if capacity == 0 {
            return HealthStatus::Healthy;
        }
        // Use f64 ratio to avoid integer division truncation issues.
        let ratio = f64::from(pending) / f64::from(capacity);
        if ratio >= 0.8 {
            HealthStatus::Critical
        } else if ratio >= 0.5 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helper constructors -------------------------------------------------

    fn idle_node(id: u32) -> ShardNode {
        ShardNode::new(id, 0, 10, 0, 0)
    }

    fn active_node(id: u32, active: u32, max_runs: u32) -> ShardNode {
        ShardNode::new(id, active, max_runs, 0, 0)
    }

    fn overloaded_node(id: u32) -> ShardNode {
        ShardNode::new(id, 10, 10, 0, 0)
    }

    fn node_with_queues(id: u32, active: u32, max_runs: u32, ready: u32, action: u32) -> ShardNode {
        ShardNode::new(id, active, max_runs, ready, action)
    }

    // -- from_shards tests ---------------------------------------------------

    #[test]
    fn from_shards_empty_is_idle_and_healthy() {
        let snap = TopologySnapshot::from_shards(Vec::new());
        assert_eq!(snap.worst_status, ShardStatus::Idle);
        assert!(snap.is_healthy());
        assert_eq!(snap.total_active_runs, 0);
        assert_eq!(snap.total_pending, 0);
        assert_eq!(snap.metrics.shards.len(), 0);
        assert!(snap.shards.is_empty());
        assert!(snap.journal_writer_status.healthy);
        assert_eq!(snap.ipc_connections, 0);
    }

    #[test]
    fn from_shards_single_idle_node() {
        let snap = TopologySnapshot::from_shards(vec![idle_node(0)]);
        assert_eq!(snap.worst_status, ShardStatus::Idle);
        assert!(snap.is_healthy());
        assert_eq!(snap.total_active_runs, 0);
        assert_eq!(snap.topology.shards.len(), 1);
        assert_eq!(snap.shards.len(), 1);
        assert_eq!(snap.shards[0].shard_id, 0);
    }

    #[test]
    fn from_shards_single_active_node() {
        let snap = TopologySnapshot::from_shards(vec![active_node(0, 5, 10)]);
        assert_eq!(snap.worst_status, ShardStatus::Active);
        assert!(snap.is_healthy());
        assert_eq!(snap.total_active_runs, 5);
    }

    #[test]
    fn from_shards_single_overloaded_node() {
        let snap = TopologySnapshot::from_shards(vec![overloaded_node(0)]);
        assert_eq!(snap.worst_status, ShardStatus::Overloaded);
        assert!(!snap.is_healthy());
        assert_eq!(snap.total_active_runs, 10);
    }

    #[test]
    fn from_shards_mixed_status_overloaded_propagates() {
        let snap = TopologySnapshot::from_shards(vec![
            idle_node(0),
            active_node(1, 5, 10),
            overloaded_node(2),
        ]);
        assert_eq!(snap.worst_status, ShardStatus::Overloaded);
        assert!(!snap.is_healthy());
    }

    #[test]
    fn from_shards_mixed_status_active_without_overloaded() {
        let snap = TopologySnapshot::from_shards(vec![idle_node(0), active_node(1, 3, 10)]);
        assert_eq!(snap.worst_status, ShardStatus::Active);
        assert!(snap.is_healthy());
    }

    // -- total_pending tests -------------------------------------------------

    #[test]
    fn total_pending_sums_ready_and_action_depths() {
        let snap = TopologySnapshot::from_shards(vec![
            node_with_queues(0, 1, 10, 5, 3),
            node_with_queues(1, 2, 10, 10, 7),
        ]);
        // shard 0: 5 + 3 = 8, shard 1: 10 + 7 = 17, total = 25
        assert_eq!(snap.total_pending, 25);
    }

    // -- summary_text tests --------------------------------------------------

    #[test]
    fn summary_text_idle_empty() {
        let snap = TopologySnapshot::from_shards(Vec::new());
        assert_eq!(snap.summary_text(), "Idle shards=0 active=0 pending=0");
    }

    #[test]
    fn summary_text_active_with_runs() {
        let snap = TopologySnapshot::from_shards(vec![active_node(0, 7, 10)]);
        assert_eq!(snap.summary_text(), "Active shards=1 active=7 pending=0");
    }

    #[test]
    fn summary_text_overloaded_with_queues() {
        let snap = TopologySnapshot::from_shards(vec![node_with_queues(0, 10, 10, 5, 3)]);
        assert_eq!(
            snap.summary_text(),
            "Overloaded shards=1 active=10 pending=8"
        );
    }

    // -- is_healthy tests ----------------------------------------------------

    #[test]
    fn is_healthy_true_for_all_idle() {
        let snap = TopologySnapshot::from_shards(vec![idle_node(0), idle_node(1)]);
        assert!(snap.is_healthy());
    }

    #[test]
    fn is_healthy_true_for_active() {
        let snap = TopologySnapshot::from_shards(vec![active_node(0, 1, 10)]);
        assert!(snap.is_healthy());
    }

    #[test]
    fn is_healthy_false_for_overloaded() {
        let snap = TopologySnapshot::from_shards(vec![overloaded_node(0)]);
        assert!(!snap.is_healthy());
    }

    // -- metrics derivation tests -------------------------------------------

    #[test]
    fn derive_metrics_overloaded_shard_is_critical() {
        let snap = TopologySnapshot::from_shards(vec![overloaded_node(0)]);
        assert_eq!(snap.metrics.shards[0].health, HealthStatus::Critical);
        assert_eq!(snap.metrics.overall_health, HealthStatus::Critical);
    }

    #[test]
    fn derive_metrics_idle_shard_is_healthy() {
        let snap = TopologySnapshot::from_shards(vec![idle_node(0)]);
        assert_eq!(snap.metrics.shards[0].health, HealthStatus::Healthy);
        assert_eq!(snap.metrics.overall_health, HealthStatus::Healthy);
    }

    #[test]
    fn derive_metrics_active_shard_queue_pressure_degraded() {
        // active_runs=5, max_runs=10, ready=4, action=2 -> pending=6
        // ratio = 6/10 = 0.6 >= 0.5 -> Degraded
        let snap = TopologySnapshot::from_shards(vec![node_with_queues(0, 5, 10, 4, 2)]);
        assert_eq!(snap.metrics.shards[0].health, HealthStatus::Degraded);
        assert_eq!(snap.metrics.overall_health, HealthStatus::Degraded);
    }

    #[test]
    fn derive_metrics_active_shard_queue_pressure_critical() {
        // active_runs=5, max_runs=10, ready=5, action=4 -> pending=9
        // ratio = 9/10 = 0.9 >= 0.8 -> Critical
        let snap = TopologySnapshot::from_shards(vec![node_with_queues(0, 5, 10, 5, 4)]);
        assert_eq!(snap.metrics.shards[0].health, HealthStatus::Critical);
        assert_eq!(snap.metrics.overall_health, HealthStatus::Critical);
    }

    #[test]
    fn derive_metrics_saturating_arithmetic_on_huge_counts() {
        let node = ShardNode {
            shard_id: 0,
            active_runs: u32::MAX,
            max_runs: u32::MAX,
            status: ShardStatus::Active,
            ready_depth: u32::MAX,
            action_depth: u32::MAX,
        };
        let snap = TopologySnapshot::from_shards(vec![node]);
        // Saturating: total_active_runs = MAX, total_pending = MAX + MAX = MAX
        assert_eq!(snap.total_active_runs, u32::MAX);
        assert_eq!(snap.total_pending, u32::MAX);
        // Metrics totals should also saturate
        assert_eq!(snap.metrics.total_active_runs, u32::MAX);
        assert_eq!(snap.metrics.total_ready_queue_depth, u32::MAX);
        assert_eq!(snap.metrics.total_action_queue_depth, u32::MAX);
    }

    #[test]
    fn derive_metrics_multiple_shards_propagate_worst_health() {
        let snap = TopologySnapshot::from_shards(vec![
            idle_node(0),
            node_with_queues(1, 5, 10, 4, 2), // Degraded
        ]);
        assert_eq!(snap.metrics.overall_health, HealthStatus::Degraded);
    }

    #[test]
    fn derive_metrics_zero_max_runs_active_shard_is_healthy() {
        // max_runs=0 means no capacity defined, should be Healthy
        let node = ShardNode::new(0, 1, 0, 5, 5);
        assert_eq!(node.status, ShardStatus::Active);
        let snap = TopologySnapshot::from_shards(vec![node]);
        assert_eq!(snap.metrics.shards[0].health, HealthStatus::Healthy);
    }

    // -- Backward compat: old TopologySnapshot shape still works -------------

    #[test]
    fn topology_snapshot_holds_shards_and_journal() {
        let snap = TopologySnapshot::from_shards(vec![idle_node(0), active_node(1, 3, 10)]);
        assert_eq!(snap.shards.len(), 2);
        assert!(snap.journal_writer_status.healthy);
        assert_eq!(snap.ipc_connections, 0);
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

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// SEVERITY: Medium
    /// DESCRIPTION: TopologySnapshot::health_for_node uses f64 division
    /// (pending / capacity) for threshold comparison. With large u32 values,
    /// f64::from is lossless, but the comparison `ratio >= 0.8` may have
    /// floating-point edge-case issues. For example, pending=4294967295 and
    /// capacity=5368709120 would give ratio ~= 0.8, but capacity > u32::MAX
    /// is impossible here. The real concern is that f64 precision at the
    /// boundary could misclassify: e.g., 4/5 = 0.8 exactly in f64 (Critical),
    /// but 4278255360/5368709120 might not be exactly 0.8 due to float
    /// representation.
    #[test]
    fn blackhat_float_threshold_boundary_for_health() {
        // Test exact boundary: pending/capacity = 0.8 exactly.
        // 4/5 = 0.8 in f64 -- should be Critical.
        let node = ShardNode {
            shard_id: 0,
            active_runs: 1,
            max_runs: 5,
            status: ShardStatus::Active,
            ready_depth: 4,
            action_depth: 0,
        };
        let snap = TopologySnapshot::from_shards(vec![node]);
        assert_eq!(
            snap.metrics.shards[0].health,
            HealthStatus::Critical,
            "4/5 = 0.8 should be Critical"
        );

        // 3/5 = 0.6 -- should be Degraded.
        let node2 = ShardNode {
            shard_id: 1,
            active_runs: 1,
            max_runs: 5,
            status: ShardStatus::Active,
            ready_depth: 3,
            action_depth: 0,
        };
        let snap2 = TopologySnapshot::from_shards(vec![node2]);
        assert_eq!(
            snap2.metrics.shards[0].health,
            HealthStatus::Degraded,
            "3/5 = 0.6 should be Degraded"
        );

        // 2/5 = 0.4 -- should be Healthy.
        let node3 = ShardNode {
            shard_id: 2,
            active_runs: 1,
            max_runs: 5,
            status: ShardStatus::Active,
            ready_depth: 2,
            action_depth: 0,
        };
        let snap3 = TopologySnapshot::from_shards(vec![node3]);
        assert_eq!(
            snap3.metrics.shards[0].health,
            HealthStatus::Healthy,
            "2/5 = 0.4 should be Healthy"
        );
    }

    /// SEVERITY: Low
    /// DESCRIPTION: TopologySnapshot::from_shards clones the entire shard list
    /// (line 60) AND iterates again for display_shards. This means the data
    /// is processed three times: once for topology, once for derive_metrics,
    /// and once for shard_displays. Not a correctness issue but unnecessary
    /// allocation for large shard counts.
    #[test]
    fn blackhat_from_shards_triple_iteration_on_input() {
        // Verify that from_shards produces consistent results despite
        // the triple iteration.
        let nodes: Vec<ShardNode> = (0..100).map(|i| ShardNode::new(i, 1, 10, i, i)).collect();
        let snap = TopologySnapshot::from_shards(nodes);
        // shards display should match topology shards in count.
        assert_eq!(snap.shards.len(), snap.topology.shards.len());
        // total_active_runs from topology should match snap field.
        assert_eq!(snap.total_active_runs, snap.topology.total_active_runs());
    }

    /// SEVERITY: Medium
    /// DESCRIPTION: When pending (ready_depth + action_depth) overflows u32
    /// via saturating_add, it saturates to u32::MAX. With max_runs=1, the
    /// ratio becomes u32::MAX / 1 = 4294967295.0 in f64, which is >= 0.8,
    //  so it's correctly classified as Critical. But if ready_depth and
    //  action_depth are large but their sum saturates, the ratio is inflated.
    //  This could misclassify a node as Critical when it's merely Degraded.
    #[test]
    fn blackhat_pending_saturation_inflates_ratio() {
        // ready_depth = u32::MAX, action_depth = 1
        // saturating_add gives u32::MAX
        // ratio = u32::MAX / 10 = very large -> Critical
        // But the "real" pending would overflow, so we can't know the true ratio.
        // The saturation causes a conservative (over-alerting) classification.
        let node = ShardNode {
            shard_id: 0,
            active_runs: 1,
            max_runs: 10,
            status: ShardStatus::Active,
            ready_depth: u32::MAX,
            action_depth: 1,
        };
        let snap = TopologySnapshot::from_shards(vec![node]);
        // Saturated pending = u32::MAX, ratio = MAX/10 -> Critical
        assert_eq!(
            snap.metrics.shards[0].health,
            HealthStatus::Critical,
            "saturated pending should classify as Critical (conservative over-alerting)"
        );
    }

    /// SEVERITY: Low
    /// DESCRIPTION: ShardNode::new considers a node Overloaded when
    /// active_runs >= max_runs AND max_runs > 0. But a node could have
    /// active_runs > max_runs if externally constructed (via struct literal).
    /// The health_for_node function doesn't re-check ShardNode::new's logic;
    /// it relies on the status field being correctly set.
    #[test]
    fn blackhat_external_shard_node_can_have_inconsistent_status() {
        // Construct a ShardNode directly with inconsistent status.
        let node = ShardNode {
            shard_id: 0,
            active_runs: 0,
            max_runs: 10,
            status: ShardStatus::Overloaded, // Inconsistent: 0 runs but Overloaded
            ready_depth: 0,
            action_depth: 0,
        };
        let snap = TopologySnapshot::from_shards(vec![node]);
        // health_for_node sees Overloaded status and returns Critical.
        assert_eq!(
            snap.metrics.shards[0].health,
            HealthStatus::Critical,
            "externally set Overloaded status propagates even with zero runs"
        );
    }

    /// SEVERITY: Low
    /// DESCRIPTION: The summary_text method formats shard_count using
    /// topology.shards.len() which returns usize. On a 64-bit platform this
    /// could be very large, producing a very long string. The format! macro
    /// will allocate accordingly. Not a real concern in practice.
    #[test]
    fn blackhat_summary_text_with_many_shards() {
        let nodes: Vec<ShardNode> = (0..1000).map(|i| ShardNode::new(i, 1, 10, 0, 0)).collect();
        let snap = TopologySnapshot::from_shards(nodes);
        let text = snap.summary_text();
        assert!(text.contains("shards=1000"));
        assert!(text.contains("active=1000"));
    }
}
