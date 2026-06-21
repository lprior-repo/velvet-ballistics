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
    let Some(capacity_u16) = bounded_u16(capacity) else {
        return 100.0;
    };
    let Some(len_u16) = bounded_u16(shard.trace_ring().len()) else {
        return 100.0;
    };
    (f32::from(len_u16) / f32::from(capacity_u16)) * 100.0
}

fn bounded_u16(value: usize) -> Option<u16> {
    u16::try_from(value).ok()
}

fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
