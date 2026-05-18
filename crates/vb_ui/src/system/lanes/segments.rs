#![forbid(unsafe_code)]
//! Lane segments and activity heatmap.

use vb_ipc::ShardMetrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RunState {
    Running,
    Waiting,
    Degraded,
    Critical,
}

impl RunState {
    #[must_use]
    pub const fn color(self) -> [f32; 4] {
        match self {
            Self::Running => [0.0, 0.961, 1.0, 1.0],
            Self::Waiting => [0.176, 0.420, 1.0, 1.0],
            Self::Degraded => [1.0, 0.902, 0.0, 1.0],
            Self::Critical => [1.0, 0.027, 0.227, 1.0],
        }
    }

    fn from_metrics_and_ratio(m: &ShardMetrics, queue_ratio: f32) -> Self {
        let pool_ratio = if m.frame_pool_total > 0 {
            f64::from(m.frame_pool_total.saturating_sub(m.frame_pool_free))
                / f64::from(m.frame_pool_total)
        } else {
            0.0
        };

        if (pool_ratio >= 0.8 || m.trace_ring_fill_pct >= 90.0) && queue_ratio >= 0.3 {
            return Self::Critical;
        }

        if (pool_ratio >= 0.5 || m.trace_ring_fill_pct >= 70.0) && queue_ratio >= 0.2 {
            return Self::Degraded;
        }

        if m.action_queue_depth > m.ready_queue_depth {
            return Self::Waiting;
        }

        Self::Running
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaneSegment {
    pub run_id: u64,
    pub width_ratio: f32,
    pub state_color: [f32; 4],
    pub label: String,
}

pub struct LaneSegmentBuilder;

const SYNTHETIC_RUN_ID_OFFSET: u64 = 80_000;

#[allow(clippy::cast_precision_loss, clippy::as_conversions)]
fn f64_to_f32(v: f64) -> f32 {
    v as f32
}

impl LaneSegmentBuilder {
    #[must_use]
    pub fn build(m: &ShardMetrics) -> Vec<LaneSegment> {
        let active = m.active_runs;
        if active == 0 {
            return Vec::new();
        }

        let active_f = f64::from(active);
        let equal_share_f64 = 1.0 / active_f;

        let capacity = match usize::try_from(active) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut segments = Vec::with_capacity(capacity);
        let mut accumulated_f64 = 0.0_f64;

        for i in 0..active {
            let run_id = SYNTHETIC_RUN_ID_OFFSET
                .saturating_add(u64::from(m.shard_id).saturating_mul(10_000))
                .saturating_add(u64::from(i));

            let is_last = i.saturating_add(1) == active;
            let width_f64 = if is_last {
                1.0 - accumulated_f64
            } else {
                equal_share_f64
            };

            let width_ratio = f64_to_f32(width_f64);

            let queue_ratio = f64_to_f32(equal_share_f64);

            let state = RunState::from_metrics_and_ratio(m, queue_ratio);

            accumulated_f64 += width_f64;

            let clamped = if width_ratio < 0.0 { 0.0 } else { width_ratio };

            segments.push(LaneSegment {
                run_id,
                width_ratio: clamped,
                state_color: state.color(),
                label: format!("R{}", run_id),
            });
        }

        segments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityHeatmap {
    buckets: Box<[u32]>,
    bucket_count: u32,
    bucket_duration_ms: u32,
    max_bucket: u32,
}

impl ActivityHeatmap {
    #[must_use]
    pub fn new(bucket_count: u32, bucket_duration_ms: u32) -> Option<Self> {
        if bucket_count == 0 || bucket_duration_ms == 0 {
            return None;
        }
        let count = match usize::try_from(bucket_count) {
            Ok(c) => c,
            Err(_) => return None,
        };
        let buckets = vec![0u32; count].into_boxed_slice();
        Some(Self {
            buckets,
            bucket_count,
            bucket_duration_ms,
            max_bucket: 0,
        })
    }

    pub fn record_event(&mut self, time_ms: u64) {
        let total_duration_ms =
            u64::from(self.bucket_count).saturating_mul(u64::from(self.bucket_duration_ms));
        let time_clamped = if time_ms >= total_duration_ms {
            total_duration_ms.saturating_sub(1)
        } else {
            time_ms
        };
        let duration = u64::from(self.bucket_duration_ms);
        let Some(bucket_idx) = time_clamped.checked_div(duration) else {
            return;
        };
        let idx = match usize::try_from(bucket_idx) {
            Ok(i) if i < self.buckets.len() => i,
            _ => return,
        };
        let Some(count) = self.buckets.get_mut(idx) else {
            return;
        };
        *count = count.saturating_add(1);
        if *count > self.max_bucket {
            self.max_bucket = *count;
        }
    }

    #[must_use]
    pub fn intensity(&self, bucket: u32) -> f32 {
        let idx = match usize::try_from(bucket) {
            Ok(i) if i < self.buckets.len() => i,
            _ => return 0.0,
        };
        let Some(&count) = self.buckets.get(idx) else {
            return 0.0;
        };
        if self.max_bucket == 0 {
            return 0.0;
        }
        f64_to_f32(f64::from(count) / f64::from(self.max_bucket))
    }
}
