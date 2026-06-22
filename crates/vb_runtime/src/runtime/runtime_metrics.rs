#![forbid(unsafe_code)]
//! Runtime facade metrics and observability helpers.

use vb_core::ids::RunId;

use crate::counters::{CounterSnapshot, RuntimeMetricsSnapshot, ShardMetricsSnapshot};
use crate::shard::Shard;
use crate::trace::TraceEvent;
use crate::{Runtime, RuntimeResult};

pub(crate) fn collect_metrics(shards_ref: &[Shard], shard_count: usize) -> RuntimeMetricsSnapshot {
    let mut shards = Vec::with_capacity(shard_count);
    let mut totals = RuntimeMetricTotals::default();
    for (index, shard) in shards_ref.iter().enumerate() {
        let metrics = shard_metrics(index, shard);
        totals.add(&metrics);
        shards.push(metrics);
    }
    totals.into_snapshot(shards)
}

#[derive(Default)]
struct RuntimeMetricTotals {
    runs_active: u32,
    runs_waiting: u32,
    runs_failed_total: u64,
    runs_finished_total: u64,
    runs_cancelled_total: u64,
    runs_killed_total: u64,
    steps_total: u64,
}

impl RuntimeMetricTotals {
    fn add(&mut self, metrics: &ShardMetricsSnapshot) {
        self.runs_active = self.runs_active.saturating_add(metrics.active_runs);
        self.runs_waiting = self.runs_waiting.saturating_add(metrics.pending_timers);
        self.runs_failed_total = self
            .runs_failed_total
            .saturating_add(metrics.counters.runs_failed);
        self.runs_finished_total = self
            .runs_finished_total
            .saturating_add(metrics.counters.runs_completed);
        self.runs_cancelled_total = self
            .runs_cancelled_total
            .saturating_add(metrics.counters.runs_cancelled);
        self.runs_killed_total = self
            .runs_killed_total
            .saturating_add(metrics.counters.runs_killed);
        self.steps_total = self
            .steps_total
            .saturating_add(metrics.counters.steps_executed);
    }

    fn into_snapshot(self, shards: Vec<ShardMetricsSnapshot>) -> RuntimeMetricsSnapshot {
        RuntimeMetricsSnapshot {
            shards,
            runs_active: self.runs_active,
            runs_waiting: self.runs_waiting,
            runs_failed_total: self.runs_failed_total,
            runs_finished_total: self.runs_finished_total,
            runs_cancelled_total: self.runs_cancelled_total,
            runs_killed_total: self.runs_killed_total,
            steps_total: self.steps_total,
        }
    }
}

impl Runtime {
    /// Lists trace events for one run without draining.
    pub fn list_events(&self, run: RunId) -> RuntimeResult<Vec<TraceEvent>> {
        let shard = self.shard_for(run)?;
        let limit = shard.trace_ring().capacity();
        Ok(shard.trace_ring().snapshot_for_run(run, limit))
    }

    /// Drains all trace events from all shards.
    pub fn drain_trace(&mut self) -> Vec<TraceEvent> {
        let mut events = Vec::new();
        for shard in &mut self.shards {
            let capacity = shard.trace_ring_mut().capacity();
            shard.trace_ring_mut().drain_into(capacity, &mut events);
        }
        events
    }

    /// Collects runtime metrics from all shards.
    pub fn collect_metrics(&self) -> RuntimeMetricsSnapshot {
        collect_metrics(&self.shards, self.shard_count)
    }

    pub fn counters_snapshot(&self) -> CounterSnapshot {
        let mut total = CounterSnapshot {
            runs_submitted: 0,
            runs_completed: 0,
            runs_failed: 0,
            runs_cancelled: 0,
            runs_killed: 0,
            steps_executed: 0,
        };
        for shard in &self.shards {
            let snap = shard.counters().snapshot();
            total.runs_submitted = total.runs_submitted.saturating_add(snap.runs_submitted);
            total.runs_completed = total.runs_completed.saturating_add(snap.runs_completed);
            total.runs_failed = total.runs_failed.saturating_add(snap.runs_failed);
            total.runs_cancelled = total.runs_cancelled.saturating_add(snap.runs_cancelled);
            total.runs_killed = total.runs_killed.saturating_add(snap.runs_killed);
            total.steps_executed = total.steps_executed.saturating_add(snap.steps_executed);
        }
        total
    }
}

fn shard_metrics(index: usize, shard: &Shard) -> ShardMetricsSnapshot {
    let counters = shard.counters().snapshot();
    let (fp_free, fp_total) = shard.frame_pool_metrics();
    ShardMetricsSnapshot {
        shard_id: saturating_u32(index),
        active_runs: saturating_u32(shard.active_run_count()),
        command_queue_depth: saturating_u32(shard.command_queue_len()),
        command_queue_remaining: saturating_u32(shard.remaining_capacity()),
        pending_timers: saturating_u32(shard.pending_timer_count()),
        frame_pool_free: saturating_u32(fp_free),
        frame_pool_total: saturating_u32(fp_total),
        trace_ring_fill_pct: trace_fill_pct(shard),
        counters,
    }
}

fn trace_fill_pct(shard: &Shard) -> f32 {
    let capacity = shard.trace_ring().capacity();
    if capacity == 0 {
        return 0.0;
    }
    let len = shard.trace_ring().len();
    // `capacity` is bounded by `MAX_TRACE_RING_CAPACITY = 1_048_576` and
    // `len <= capacity`, so both values fit comfortably in `u32`. Computing
    // the ratio in `f64` (rather than bounding through `u16` as the previous
    // implementation did) preserves precision across the full legal capacity
    // range. The previous `bounded_u16` branch saturated any ring whose
    // capacity exceeded `u16::MAX` (65 535) at `100.0`, masking actual fill
    // ratios up to `MAX_TRACE_RING_CAPACITY`.
    let capacity_u32 = u32::try_from(capacity).unwrap_or(u32::MAX);
    let len_u32 = u32::try_from(len).unwrap_or(u32::MAX);
    let ratio_f64 = f64::from(len_u32) / f64::from(capacity_u32);
    let pct_f64 = ratio_f64 * 100.0;
    // Narrow `pct_f64` (always in `[0.0, 100.0]` since `len <= capacity`)
    // to `f32`. The saturating cast is preferred here because `f32` has no
    // `From<f64>` impl and the `TryFrom<f64>` impl requires it. The cast is
    // exact for every reachable input: f32 represents every integer in
    // `[0, 2^24]` exactly, and `pct_f64` is bounded by `100.0`.
    #[allow(clippy::as_conversions)]
    let pct_f32 = pct_f64 as f32;
    pct_f32
}

fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    //! Regression coverage for RA-003 (`trace_fill_pct` saturation bug).
    //!
    //! Before the fix, `trace_fill_pct` saturated at `100.0` for any trace
    //! ring whose capacity exceeded `u16::MAX`. With production trace rings
    //! configured up to `MAX_TRACE_RING_CAPACITY = 1_048_576`, that covered a
    //! large live configuration range. These tests pin the corrected
    //! behaviour: empty rings with capacities above `u16::MAX` must report
    //! `0.0`, and partial fills must report the actual ratio.

    use std::num::NonZeroUsize;

    use vb_core::ids::RunId;
    use vb_core::limits::MAX_TRACE_RING_CAPACITY;
    use vb_core::policy::RuntimePolicy;

    use super::Runtime;
    use crate::shard::ShardConfig;
    use crate::trace::TraceEvent;

    fn config_with_trace_capacity(trace_capacity: usize) -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
            snapshot_interval_steps: 0,
            max_terminal_runs: 16,
            terminal_runs_ttl_ticks: 86_400,
            max_terminal_outcomes: 100_000,
        }
    }

    fn runtime_with_trace_capacity(trace_capacity: usize) -> Runtime {
        let shard_count = NonZeroUsize::new(1).expect("non-zero");
        Runtime::new(shard_count, config_with_trace_capacity(trace_capacity))
            .expect("runtime construction should succeed with valid capacity")
    }

    fn push_events(runtime: &mut Runtime, count: usize) {
        let event = TraceEvent::RunSubmitted { run: RunId::new(1) };
        for _ in 0..count {
            let shard = &mut runtime.shards[0];
            if !shard.trace_ring_mut().push(event.clone()) {
                // Ring reached capacity; stop pushing.
                return;
            }
        }
    }

    #[test]
    fn trace_fill_pct_zero_when_empty_at_u16_max_boundary() {
        // Capacity == u16::MAX (65_535) and length == 0 must report 0.0.
        // This is the boundary case just below where the old implementation
        // started saturating at 100.0.
        let runtime = runtime_with_trace_capacity(u16::MAX as usize);
        let pct = runtime.collect_metrics().shards[0].trace_ring_fill_pct;
        assert_eq!(pct, 0.0, "empty ring at u16::MAX must report 0%, got {pct}");
    }

    #[test]
    fn trace_fill_pct_zero_when_empty_just_above_u16_max() {
        // REGRESSION (RA-003): capacity == u16::MAX + 1 (65_536) with length
        // 0 must report 0.0, not 100.0. Previously the `bounded_u16` branch
        // saturated at 100% for any capacity above `u16::MAX`.
        let runtime = runtime_with_trace_capacity(usize::from(u16::MAX) + 1);
        let pct = runtime.collect_metrics().shards[0].trace_ring_fill_pct;
        assert_eq!(
            pct, 0.0,
            "empty ring with capacity > u16::MAX must report 0%, got {pct}"
        );
    }

    #[test]
    fn trace_fill_pct_zero_when_empty_at_max_trace_ring_capacity() {
        // REGRESSION (RA-003): capacity at the production ceiling
        // `MAX_TRACE_RING_CAPACITY = 1_048_576` with length 0 must report
        // 0.0, not 100.0. This is the worst-case configuration where the
        // original bug was most misleading.
        let runtime = runtime_with_trace_capacity(MAX_TRACE_RING_CAPACITY);
        let pct = runtime.collect_metrics().shards[0].trace_ring_fill_pct;
        assert_eq!(
            pct, 0.0,
            "empty ring at MAX_TRACE_RING_CAPACITY must report 0%, got {pct}"
        );
    }

    #[test]
    fn trace_fill_pct_reports_actual_ratio_above_u16_max() {
        // Capacity 100_000 (well above u16::MAX) with 25_000 events pushed
        // must report ~25.0%. The previous implementation would have reported
        // 100.0 here regardless of fill.
        let mut runtime = runtime_with_trace_capacity(100_000);
        push_events(&mut runtime, 25_000);
        let pct = runtime.collect_metrics().shards[0].trace_ring_fill_pct;
        let expected = 25.0_f32;
        let delta = (pct - expected).abs();
        assert!(
            delta < 0.01,
            "expected fill ratio near {expected}% but got {pct} (delta {delta})"
        );
    }

    #[test]
    fn trace_fill_pct_reports_100_when_full_at_capacity_above_u16_max() {
        // Filling a 100_000-capacity ring completely must report 100.0.
        let mut runtime = runtime_with_trace_capacity(100_000);
        push_events(&mut runtime, 100_000);
        let pct = runtime.collect_metrics().shards[0].trace_ring_fill_pct;
        assert_eq!(
            pct, 100.0,
            "full ring with capacity > u16::MAX must report 100%, got {pct}"
        );
    }

    #[test]
    fn trace_fill_pct_reports_correct_fill_at_max_trace_ring_capacity() {
        // Half-filling `MAX_TRACE_RING_CAPACITY` must report 50.0%, not 100%.
        let mut runtime = runtime_with_trace_capacity(MAX_TRACE_RING_CAPACITY);
        let half = MAX_TRACE_RING_CAPACITY / 2;
        push_events(&mut runtime, half);
        let pct = runtime.collect_metrics().shards[0].trace_ring_fill_pct;
        let expected = 50.0_f32;
        let delta = (pct - expected).abs();
        assert!(
            delta < 0.01,
            "expected fill ratio near {expected}% at MAX_TRACE_RING_CAPACITY, got {pct} (delta {delta})"
        );
    }

    #[test]
    fn trace_fill_pct_zero_when_empty_at_small_capacity() {
        // Baseline: small-capacity empty ring still reports 0.0 (no
        // regression at the low end).
        let runtime = runtime_with_trace_capacity(16);
        let pct = runtime.collect_metrics().shards[0].trace_ring_fill_pct;
        assert_eq!(pct, 0.0, "empty ring at capacity 16 must report 0%");
    }
}
