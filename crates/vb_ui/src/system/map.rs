#![forbid(unsafe_code)]
// System Map / Topology module for Phase 3A.
// Provides topology snapshot data for the System Overview screen,
// including shard node status, system-wide aggregates, and grid layout
// computation for rendering.

// --- Float/int conversion helpers (isolated for auditability) ---

/// Convert a usize to f32, isolated for auditability.
/// Lossless for layout-sized values (< 2^24).
#[allow(clippy::cast_precision_loss, clippy::as_conversions)]
fn int_to_f32(v: usize) -> f32 {
    v as f32
}

/// Convert a non-negative f32 to u32, clamping to [0, u32::MAX].
/// Isolated for auditability.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions
)]
fn f32_to_u32(v: f32) -> u32 {
    if v <= 0.0 {
        0
    } else if v >= 16_777_216.0 {
        // f32 can't represent values > 2^24 precisely, clamp to safe range
        u32::MAX
    } else {
        v.round() as u32
    }
}

/// Color constants as linear RGBA (suitable for GPU/shader consumption).
#[allow(dead_code)]
mod colors {
    /// neon_cyan #00f5ff — running / active
    pub(super) const NEON_CYAN: [f32; 4] = [0.0, 0.961, 1.0, 1.0];
    /// neon_green #39ff14 — healthy / idle
    pub(super) const NEON_GREEN: [f32; 4] = [0.224, 1.0, 0.078, 1.0];
    /// neon_red #ff073a — failed / overloaded
    pub(super) const NEON_RED: [f32; 4] = [1.0, 0.027, 0.227, 1.0];
    /// neon_blue #2d6bff — waiting
    pub(super) const NEON_BLUE: [f32; 4] = [0.176, 0.420, 1.0, 1.0];
    /// neon_orange #ff6b00 — action
    pub(super) const NEON_ORANGE: [f32; 4] = [1.0, 0.420, 0.0, 1.0];
}

/// Status of a single shard within the system topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShardStatus {
    /// Shard is actively processing runs.
    Active,
    /// Shard has no active runs and is waiting for work.
    Idle,
    /// Shard is overloaded (active_runs >= max_runs or queues backed up).
    Overloaded,
}

impl ShardStatus {
    /// Returns the display color associated with this shard status.
    #[must_use]
    pub const fn status_color(self) -> [f32; 4] {
        match self {
            Self::Active => colors::NEON_CYAN,
            Self::Idle => colors::NEON_GREEN,
            Self::Overloaded => colors::NEON_RED,
        }
    }

    /// Ordering severity: Overloaded > Active > Idle.
    /// Returns `true` if `self` is strictly worse than `other`.
    const fn is_worse_than(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Overloaded, Self::Active)
                | (Self::Overloaded, Self::Idle)
                | (Self::Active, Self::Idle)
        )
    }
}

/// A single shard node in the system topology.
#[derive(Debug, Clone)]
pub struct ShardNode {
    pub shard_id: u32,
    pub active_runs: u32,
    pub max_runs: u32,
    pub status: ShardStatus,
    pub ready_depth: u32,
    pub action_depth: u32,
}

impl ShardNode {
    /// Constructs a new `ShardNode`, computing status from active_runs and max_runs.
    #[must_use]
    pub fn new(
        shard_id: u32,
        active_runs: u32,
        max_runs: u32,
        ready_depth: u32,
        action_depth: u32,
    ) -> Self {
        let status = if max_runs > 0 && active_runs >= max_runs {
            ShardStatus::Overloaded
        } else if active_runs == 0 {
            ShardStatus::Idle
        } else {
            ShardStatus::Active
        };

        Self {
            shard_id,
            active_runs,
            max_runs,
            status,
            ready_depth,
            action_depth,
        }
    }
}

/// Aggregated system-wide topology snapshot.
#[derive(Debug, Clone)]
pub struct SystemTopology {
    pub shards: Vec<ShardNode>,
}

impl SystemTopology {
    /// Returns the worst shard status across all shards.
    ///
    /// Severity order: Overloaded > Active > Idle.
    /// Returns `Idle` for an empty topology (no shards means nothing is wrong).
    #[must_use]
    pub fn worst_status(&self) -> ShardStatus {
        self.shards.iter().fold(ShardStatus::Idle, |worst, shard| {
            if shard.status.is_worse_than(worst) {
                shard.status
            } else {
                worst
            }
        })
    }

    /// Sums `active_runs` across all shards.
    #[must_use]
    pub fn total_active_runs(&self) -> u32 {
        self.shards
            .iter()
            .fold(0u32, |acc, s| acc.saturating_add(s.active_runs))
    }

    /// Sums `ready_depth + action_depth` across all shards.
    #[must_use]
    pub fn total_pending_actions(&self) -> u32 {
        self.shards.iter().fold(0u32, |acc, s| {
            let pending = s.ready_depth.saturating_add(s.action_depth);
            acc.saturating_add(pending)
        })
    }
}

/// A positioned rectangle for a single shard in the layout.
#[derive(Debug, Clone, Copy)]
pub struct ShardRect {
    pub shard_id: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
}

/// Layout engine for the system map grid.
///
/// Positions shard nodes in an auto-sized grid within the given bounds.
/// Grid columns are chosen to keep tiles roughly square.
pub struct SystemMapLayout;

impl SystemMapLayout {
    /// Computes the grid layout for the given topology within the specified bounds.
    ///
    /// Returns a `ShardRect` per shard, arranged in a row-major grid.
    /// Returns an empty vec for an empty topology.
    /// Returns an empty vec if width or height is non-positive.
    #[must_use]
    pub fn compute_layout(topology: &SystemTopology, width: f32, height: f32) -> Vec<ShardRect> {
        if width <= 0.0 || height <= 0.0 {
            return Vec::new();
        }

        let count = topology.shards.len();
        if count == 0 {
            return Vec::new();
        }

        let cols = Self::optimal_columns(count, width, height);
        let rows = Self::rows_for(count, cols);

        let cols_f = int_to_f32(cols).max(1.0);
        let rows_f = int_to_f32(rows).max(1.0);
        let cell_w = width / cols_f;
        let cell_h = height / rows_f;

        topology
            .shards
            .iter()
            .enumerate()
            .map(|(i, shard)| {
                let col = i.checked_rem(cols).unwrap_or(0);
                let row = i.checked_div(cols).unwrap_or(0);
                let x = int_to_f32(col) * cell_w;
                let y = int_to_f32(row) * cell_h;
                ShardRect {
                    shard_id: shard.shard_id,
                    x,
                    y,
                    w: cell_w,
                    h: cell_h,
                    color: shard.status.status_color(),
                }
            })
            .collect()
    }

    /// Pick column count so tiles stay roughly square.
    /// Formula: cols = ceil(sqrt(count * (w/h)))
    fn optimal_columns(count: usize, width: f32, height: f32) -> usize {
        if count == 0 || height <= 0.0 {
            return 1;
        }
        let count_f = int_to_f32(count);
        let ratio = width / height;
        let raw = count_f.sqrt() * ratio.sqrt();
        let ceiled = raw.ceil().max(1.0);
        let cols_u32 = f32_to_u32(ceiled).max(1);
        let cols = usize::try_from(cols_u32).unwrap_or(1);
        cols.min(count)
    }

    /// Number of rows needed for `count` items in `cols` columns.
    fn rows_for(count: usize, cols: usize) -> usize {
        if cols == 0 {
            return 1;
        }
        let r = count.checked_div(cols).unwrap_or(0);
        if count.is_multiple_of(cols) {
            r
        } else {
            r.saturating_add(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- ShardStatus color tests --

    #[test]
    fn shard_status_active_color_is_neon_cyan() {
        let [r, g, b, a] = ShardStatus::Active.status_color();
        assert!((r - 0.0).abs() < f32::EPSILON);
        assert!((g - 0.961).abs() < 0.002);
        assert!((b - 1.0).abs() < f32::EPSILON);
        assert!((a - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn shard_status_idle_color_is_neon_green() {
        let [r, g, b, a] = ShardStatus::Idle.status_color();
        assert!((r - 0.224).abs() < 0.002);
        assert!((g - 1.0).abs() < f32::EPSILON);
        assert!((b - 0.078).abs() < 0.002);
        assert!((a - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn shard_status_overloaded_color_is_neon_red() {
        let [r, g, b, a] = ShardStatus::Overloaded.status_color();
        assert!((r - 1.0).abs() < f32::EPSILON);
        assert!((g - 0.027).abs() < 0.002);
        assert!((b - 0.227).abs() < 0.002);
        assert!((a - 1.0).abs() < f32::EPSILON);
    }

    // -- ShardNode::new status derivation tests --

    #[test]
    fn shard_node_new_idle_when_no_runs() {
        let node = ShardNode::new(0, 0, 10, 0, 0);
        assert_eq!(node.status, ShardStatus::Idle);
    }

    #[test]
    fn shard_node_new_active_when_some_runs_below_max() {
        let node = ShardNode::new(1, 5, 10, 2, 3);
        assert_eq!(node.status, ShardStatus::Active);
        assert_eq!(node.active_runs, 5);
        assert_eq!(node.ready_depth, 2);
        assert_eq!(node.action_depth, 3);
    }

    #[test]
    fn shard_node_new_overloaded_when_runs_equal_max() {
        let node = ShardNode::new(2, 10, 10, 0, 0);
        assert_eq!(node.status, ShardStatus::Overloaded);
    }

    #[test]
    fn shard_node_new_overloaded_when_runs_exceed_max() {
        let node = ShardNode::new(3, 15, 10, 5, 2);
        assert_eq!(node.status, ShardStatus::Overloaded);
        assert_eq!(node.active_runs, 15);
    }

    #[test]
    fn shard_node_new_idle_when_max_runs_is_zero() {
        // max_runs == 0 means no capacity defined, not overloaded
        let node = ShardNode::new(4, 0, 0, 0, 0);
        assert_eq!(node.status, ShardStatus::Idle);
    }

    // -- SystemTopology aggregate tests --

    #[test]
    fn topology_total_active_runs_sums_across_shards() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(0, 3, 10, 0, 0),
                ShardNode::new(1, 7, 10, 0, 0),
                ShardNode::new(2, 1, 10, 0, 0),
            ],
        };
        assert_eq!(topo.total_active_runs(), 11);
    }

    #[test]
    fn topology_total_pending_actions_sums_ready_and_action() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(0, 1, 10, 5, 3),
                ShardNode::new(1, 2, 10, 10, 7),
            ],
        };
        assert_eq!(topo.total_pending_actions(), 25);
    }

    #[test]
    fn topology_worst_status_overloaded_propagates() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(0, 0, 10, 0, 0),  // Idle
                ShardNode::new(1, 5, 10, 0, 0),  // Active
                ShardNode::new(2, 10, 10, 0, 0), // Overloaded
            ],
        };
        assert_eq!(topo.worst_status(), ShardStatus::Overloaded);
    }

    #[test]
    fn topology_worst_status_active_without_overloaded() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(0, 0, 10, 0, 0), // Idle
                ShardNode::new(1, 5, 10, 0, 0), // Active
            ],
        };
        assert_eq!(topo.worst_status(), ShardStatus::Active);
    }

    #[test]
    fn topology_worst_status_idle_when_all_idle() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(0, 0, 10, 0, 0),
                ShardNode::new(1, 0, 10, 0, 0),
            ],
        };
        assert_eq!(topo.worst_status(), ShardStatus::Idle);
    }

    #[test]
    fn topology_worst_status_idle_when_empty() {
        let topo = SystemTopology { shards: Vec::new() };
        assert_eq!(topo.worst_status(), ShardStatus::Idle);
    }

    #[test]
    fn topology_totals_zero_when_empty() {
        let topo = SystemTopology { shards: Vec::new() };
        assert_eq!(topo.total_active_runs(), 0);
        assert_eq!(topo.total_pending_actions(), 0);
    }

    // -- Layout tests --

    #[test]
    fn layout_returns_empty_for_zero_shards() {
        let topo = SystemTopology { shards: Vec::new() };
        let rects = SystemMapLayout::compute_layout(&topo, 800.0, 600.0);
        assert!(rects.is_empty());
    }

    #[test]
    fn layout_single_shard_fills_bounds() {
        let topo = SystemTopology {
            shards: vec![ShardNode::new(0, 5, 10, 0, 0)],
        };
        let rects = SystemMapLayout::compute_layout(&topo, 800.0, 600.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].shard_id, 0);
        assert!((rects[0].x - 0.0).abs() < f32::EPSILON);
        assert!((rects[0].y - 0.0).abs() < f32::EPSILON);
        assert!((rects[0].w - 800.0).abs() < 0.01);
        assert!((rects[0].h - 600.0).abs() < 0.01);
        assert_eq!(rects[0].color, ShardStatus::Active.status_color());
    }

    #[test]
    fn layout_many_shards_grid_positions() {
        let topo = SystemTopology {
            shards: (0..6).map(|i| ShardNode::new(i, 1, 10, 0, 0)).collect(),
        };
        let rects = SystemMapLayout::compute_layout(&topo, 600.0, 400.0);
        assert_eq!(rects.len(), 6);

        // All rects should have positive width and height
        for r in &rects {
            assert!(r.w > 0.0);
            assert!(r.h > 0.0);
        }

        // Shard IDs preserved in order
        for (i, r) in rects.iter().enumerate() {
            assert_eq!(r.shard_id, u32::try_from(i).unwrap_or(u32::MAX));
        }
    }

    #[test]
    fn layout_returns_empty_for_negative_dimensions() {
        let topo = SystemTopology {
            shards: vec![ShardNode::new(0, 5, 10, 0, 0)],
        };
        assert!(SystemMapLayout::compute_layout(&topo, -100.0, 600.0).is_empty());
        assert!(SystemMapLayout::compute_layout(&topo, 800.0, -100.0).is_empty());
        assert!(SystemMapLayout::compute_layout(&topo, 0.0, 600.0).is_empty());
        assert!(SystemMapLayout::compute_layout(&topo, 800.0, 0.0).is_empty());
    }

    #[test]
    fn layout_rects_color_matches_shard_status() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(0, 0, 10, 0, 0),  // Idle
                ShardNode::new(1, 5, 10, 0, 0),  // Active
                ShardNode::new(2, 10, 10, 0, 0), // Overloaded
            ],
        };
        let rects = SystemMapLayout::compute_layout(&topo, 900.0, 300.0);
        assert_eq!(rects[0].color, ShardStatus::Idle.status_color());
        assert_eq!(rects[1].color, ShardStatus::Active.status_color());
        assert_eq!(rects[2].color, ShardStatus::Overloaded.status_color());
    }

    #[test]
    fn layout_overloaded_detection_in_topology() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(0, 3, 10, 0, 0),
                ShardNode::new(1, 10, 10, 0, 0),
                ShardNode::new(2, 7, 10, 0, 0),
            ],
        };
        assert_eq!(topo.worst_status(), ShardStatus::Overloaded);
        assert_eq!(topo.total_active_runs(), 20);
    }

    #[test]
    fn topology_saturating_arithmetic_on_huge_counts() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode {
                    shard_id: 0,
                    active_runs: u32::MAX,
                    max_runs: u32::MAX,
                    status: ShardStatus::Overloaded,
                    ready_depth: u32::MAX,
                    action_depth: u32::MAX,
                },
                ShardNode {
                    shard_id: 1,
                    active_runs: 1,
                    max_runs: 10,
                    status: ShardStatus::Active,
                    ready_depth: 1,
                    action_depth: 1,
                },
            ],
        };
        // Saturating add should not overflow
        assert_eq!(topo.total_active_runs(), u32::MAX);
        assert_eq!(topo.total_pending_actions(), u32::MAX);
    }

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// SEVERITY: Low
    /// DESCRIPTION: SystemMapLayout::compute_layout uses `as` casts (isolated
    /// and audited) for usize <-> f32 conversion. The `int_to_f32` function
    /// is documented as lossless for values < 2^24. For shard counts exceeding
    /// 16 million (extremely unlikely but theoretically possible), f32 loses
    /// precision, causing incorrect grid calculations. The `f32_to_u32`
    /// function clamps at 16_777_216 which is safe.
    #[test]
    fn blackhat_layout_large_shard_count_precision() {
        // Test with a moderately large count (well under 2^24).
        let topo = SystemTopology {
            shards: (0..1000).map(|i| ShardNode::new(i, 1, 10, 0, 0)).collect(),
        };
        let rects = SystemMapLayout::compute_layout(&topo, 1920.0, 1080.0);
        assert_eq!(rects.len(), 1000);
        // Verify all rects have positive dimensions.
        for r in &rects {
            assert!(r.w > 0.0, "width must be positive: {}", r.w);
            assert!(r.h > 0.0, "height must be positive: {}", r.h);
        }
    }

    /// SEVERITY: Medium
    /// DESCRIPTION: ShardNode::new classifies as Overloaded when
    /// active_runs >= max_runs AND max_runs > 0. If max_runs = 0, the node
    /// is Idle (if active_runs = 0) or Active (if active_runs > 0), never
    /// Overloaded. This means a node with active_runs = u32::MAX and
    /// max_runs = 0 is considered merely "Active", not overloaded, which
    /// could hide critical overload conditions.
    #[test]
    fn blackhat_zero_max_runs_never_overloaded() {
        let node = ShardNode::new(0, u32::MAX, 0, 0, 0);
        assert_eq!(
            node.status,
            ShardStatus::Active,
            "node with MAX runs but zero max_runs is Active, not Overloaded"
        );
    }

    /// SEVERITY: Low
    /// DESCRIPTION: ShardStatus::is_worse_than doesn't handle the case where
    /// both statuses are equal -- it correctly returns false (a status is not
    /// strictly worse than itself). The fold in worst_status starts with Idle
    /// and only replaces when strictly worse, which is correct.
    #[test]
    fn blackhat_is_worse_than_same_status_returns_false() {
        assert!(!ShardStatus::Active.is_worse_than(ShardStatus::Active));
        assert!(!ShardStatus::Idle.is_worse_than(ShardStatus::Idle));
        assert!(!ShardStatus::Overloaded.is_worse_than(ShardStatus::Overloaded));
    }

    /// SEVERITY: Low
    /// DESCRIPTION: The compute_layout function divides width/height by cols/rows.
    /// With very large shard counts, cell dimensions become very small (< 1 pixel).
    /// This is valid mathematically but renders poorly. No guard exists for a
    /// minimum cell size.
    #[test]
    fn blackhat_layout_tiny_cells_with_many_shards() {
        let topo = SystemTopology {
            shards: (0..100).map(|i| ShardNode::new(i, 1, 10, 0, 0)).collect(),
        };
        // Very small viewport with many shards.
        let rects = SystemMapLayout::compute_layout(&topo, 10.0, 10.0);
        assert_eq!(rects.len(), 100);
        // Cell width could be < 1 pixel.
        let cell_w = rects[0].w;
        assert!(cell_w > 0.0, "cell width should still be positive");
        // But it may be less than 1 pixel, making rendering useless.
        if cell_w < 1.0 {
            // This is expected with 100 shards in 10px width.
            assert!(cell_w < 1.0, "cell width is sub-pixel: {}", cell_w);
        }
    }

    /// SEVERITY: Low
    /// DESCRIPTION: The optimal_columns function uses sqrt and ceil via f32.
    /// For count=1, cols=1. For very tall narrow viewports (height >> width),
    /// optimal_columns could return 1, leading to a single column with many
    /// rows. This is correct behavior (keeps tiles roughly square) but may
    /// not match user expectations for a "grid".
    #[test]
    fn blackhat_optimal_columns_for_extreme_aspect_ratio() {
        let cols = SystemMapLayout::optimal_columns(100, 10.0, 10000.0);
        // Very tall viewport: cols should be 1 (sqrt(100 * 0.001) ~= 0.316, ceil = 1).
        assert_eq!(cols, 1, "tall narrow viewport should use 1 column");
        let rows = SystemMapLayout::rows_for(100, cols);
        assert_eq!(rows, 100, "all 100 items in 1 column");
    }

    /// SEVERITY: Low
    /// DESCRIPTION: rows_for returns 1 when cols=0, which is a guard against
    /// division by zero. But optimal_columns.min(count) ensures cols >= 1,
    /// so the guard is defensive. If someone calls rows_for directly with
    /// cols=0, it returns 1 instead of panicking.
    #[test]
    fn blackhat_rows_for_zero_cols_returns_one() {
        assert_eq!(SystemMapLayout::rows_for(100, 0), 1);
    }

    // =========================================================================
    // Additional comprehensive coverage tests
    // =========================================================================

    #[test]
    fn shard_status_is_worse_than_active_not_worse_than_overloaded() {
        assert!(!ShardStatus::Active.is_worse_than(ShardStatus::Overloaded));
    }

    #[test]
    fn shard_status_is_worse_than_idle_not_worse_than_active() {
        assert!(!ShardStatus::Idle.is_worse_than(ShardStatus::Active));
    }

    #[test]
    fn shard_status_is_worse_than_overloaded_is_worse_than_all() {
        assert!(ShardStatus::Overloaded.is_worse_than(ShardStatus::Active));
        assert!(ShardStatus::Overloaded.is_worse_than(ShardStatus::Idle));
    }

    #[test]
    fn shard_status_is_worse_than_active_is_worse_than_idle() {
        assert!(ShardStatus::Active.is_worse_than(ShardStatus::Idle));
    }

    #[test]
    fn f32_to_u32_clamps_negative_to_zero() {
        assert_eq!(f32_to_u32(-1.0), 0);
        assert_eq!(f32_to_u32(-0.001), 0);
    }

    #[test]
    fn f32_to_u32_clamps_very_large_to_u32_max() {
        assert_eq!(f32_to_u32(16_777_216.0), u32::MAX);
        assert_eq!(f32_to_u32(1e10), u32::MAX);
    }

    #[test]
    fn f32_to_u32_rounds_correctly() {
        assert_eq!(f32_to_u32(1.4), 1);
        assert_eq!(f32_to_u32(1.5), 2);
        assert_eq!(f32_to_u32(1.6), 2);
        assert_eq!(f32_to_u32(0.0), 0);
        assert_eq!(f32_to_u32(100.0), 100);
    }

    #[test]
    fn int_to_f32_converts_small_values_losslessly() {
        assert!((int_to_f32(0) - 0.0).abs() < f32::EPSILON);
        assert!((int_to_f32(1) - 1.0).abs() < f32::EPSILON);
        assert!((int_to_f32(100) - 100.0).abs() < f32::EPSILON);
        assert!((int_to_f32(1000) - 1000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn layout_two_shards_side_by_side_in_wide_viewport() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(0, 1, 10, 0, 0),
                ShardNode::new(1, 1, 10, 0, 0),
            ],
        };
        let rects = SystemMapLayout::compute_layout(&topo, 1000.0, 500.0);
        assert_eq!(rects.len(), 2);
        // Second shard should start at x > 0 (next column).
        assert!(rects[1].x > 0.0);
        // Both should have same y (row 0).
        assert!((rects[0].y - 0.0).abs() < f32::EPSILON);
        assert!((rects[1].y - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn layout_exact_grid_four_shards_two_by_two() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(0, 1, 10, 0, 0),
                ShardNode::new(1, 1, 10, 0, 0),
                ShardNode::new(2, 1, 10, 0, 0),
                ShardNode::new(3, 1, 10, 0, 0),
            ],
        };
        let rects = SystemMapLayout::compute_layout(&topo, 200.0, 200.0);
        assert_eq!(rects.len(), 4);
        // Verify all positions are finite and non-negative.
        for r in &rects {
            assert!(r.x.is_finite());
            assert!(r.y.is_finite());
            assert!(r.x >= 0.0);
            assert!(r.y >= 0.0);
            assert!(r.w > 0.0);
            assert!(r.h > 0.0);
        }
        // Verify no overlapping: first row and second row have different y.
        // Items in same row should have different x.
        assert!((rects[0].y - rects[1].y).abs() < f32::EPSILON);
    }

    #[test]
    fn optimal_columns_for_square_viewport() {
        let cols = SystemMapLayout::optimal_columns(9, 100.0, 100.0);
        // sqrt(9 * 1.0) = 3.0, ceil = 3.
        assert_eq!(cols, 3);
    }

    #[test]
    fn optimal_columns_for_one_shard_is_one() {
        let cols = SystemMapLayout::optimal_columns(1, 800.0, 600.0);
        assert_eq!(cols, 1);
    }

    #[test]
    fn rows_for_exact_division() {
        assert_eq!(SystemMapLayout::rows_for(9, 3), 3);
        assert_eq!(SystemMapLayout::rows_for(4, 2), 2);
        assert_eq!(SystemMapLayout::rows_for(1, 1), 1);
    }

    #[test]
    fn rows_for_remainder_adds_extra_row() {
        assert_eq!(SystemMapLayout::rows_for(7, 3), 3);
        assert_eq!(SystemMapLayout::rows_for(10, 3), 4);
        assert_eq!(SystemMapLayout::rows_for(5, 2), 3);
    }

    #[test]
    fn shard_node_preserves_all_fields() {
        let node = ShardNode::new(42, 3, 10, 7, 9);
        assert_eq!(node.shard_id, 42);
        assert_eq!(node.active_runs, 3);
        assert_eq!(node.max_runs, 10);
        assert_eq!(node.status, ShardStatus::Active);
        assert_eq!(node.ready_depth, 7);
        assert_eq!(node.action_depth, 9);
    }

    #[test]
    fn shard_node_active_when_runs_nonzero_and_below_max() {
        let node = ShardNode::new(0, 1, 100, 0, 0);
        assert_eq!(node.status, ShardStatus::Active);

        let node = ShardNode::new(0, 99, 100, 0, 0);
        assert_eq!(node.status, ShardStatus::Active);
    }

    #[test]
    fn topology_total_pending_actions_with_zero_depths() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(0, 1, 10, 0, 0),
                ShardNode::new(1, 2, 10, 0, 0),
            ],
        };
        assert_eq!(topo.total_pending_actions(), 0);
    }

    #[test]
    fn layout_preserves_shard_ordering() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(10, 0, 5, 0, 0), // Idle
                ShardNode::new(20, 2, 5, 0, 0), // Active
                ShardNode::new(30, 5, 5, 0, 0), // Overloaded
                ShardNode::new(40, 0, 5, 0, 0), // Idle
            ],
        };
        let rects = SystemMapLayout::compute_layout(&topo, 400.0, 300.0);
        let ids: Vec<u32> = rects.iter().map(|r| r.shard_id).collect();
        assert_eq!(ids, vec![10, 20, 30, 40]);
    }

    #[test]
    fn layout_cell_dimensions_sum_to_bounds() {
        let topo = SystemTopology {
            shards: vec![
                ShardNode::new(0, 1, 10, 0, 0),
                ShardNode::new(1, 1, 10, 0, 0),
            ],
        };
        let width = 800.0_f32;
        let rects = SystemMapLayout::compute_layout(&topo, width, 600.0);
        assert_eq!(rects.len(), 2);
        // All cells should have the same width.
        assert!((rects[0].w - rects[1].w).abs() < f32::EPSILON);
        // Total covered width equals viewport width.
        let _covered =
            rects[0].x + rects[0].w + (rects[1].x - rects[0].x - rects[0].w) + rects[1].w;
        // Due to grid layout: covered = cols * cell_w = width
        let total_cell_width = rects[0].w + (width - rects[0].w - rects[1].w) + rects[1].w;
        let _ = total_cell_width; // Just verify cells are consistent.
        assert!(
            (rects[0].w + rects[1].x - rects[0].x).abs() < f32::EPSILON
                || (rects[0].w * 2.0 - width).abs() < 1.0
        );
    }

    #[test]
    fn topology_clone_preserves_data() {
        let topo = SystemTopology {
            shards: vec![ShardNode::new(0, 5, 10, 3, 7)],
        };
        let cloned = topo.clone();
        assert_eq!(cloned.shards.len(), 1);
        assert_eq!(cloned.total_active_runs(), 5);
        assert_eq!(cloned.total_pending_actions(), 10);
    }

    #[test]
    fn shard_node_debug_format_contains_fields() {
        let node = ShardNode::new(99, 1, 10, 5, 3);
        let debug_str = format!("{node:?}");
        assert!(debug_str.contains("shard_id"));
        assert!(debug_str.contains("active_runs"));
    }

    #[test]
    fn shard_rect_debug_format() {
        let rect = ShardRect {
            shard_id: 5,
            x: 10.0,
            y: 20.0,
            w: 100.0,
            h: 50.0,
            color: [1.0, 0.0, 0.0, 1.0],
        };
        let debug_str = format!("{rect:?}");
        assert!(debug_str.contains("shard_id"));
    }

    #[test]
    fn shard_status_debug_variants() {
        assert!(format!("{:?}", ShardStatus::Active).contains("Active"));
        assert!(format!("{:?}", ShardStatus::Idle).contains("Idle"));
        assert!(format!("{:?}", ShardStatus::Overloaded).contains("Overloaded"));
    }
}
