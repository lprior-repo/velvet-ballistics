#![forbid(unsafe_code)]

use crate::counters::{CounterSnapshot, RuntimeMetricsSnapshot, ShardMetricsSnapshot};
use crate::runtime::{ActiveRunSummary, Runtime};
use crate::shard::{RunState, Shard};
use vb_core::frame::StepState;
use vb_core::ids::{RunId, StepIdx};

#[derive(Default)]
struct MetricsTotals {
    runs_active: u32,
    runs_waiting: u32,
    runs_failed_total: u64,
    runs_finished_total: u64,
    steps_total: u64,
}

impl MetricsTotals {
    fn add_shard(self, snapshot: &ShardMetricsSnapshot) -> Self {
        Self {
            runs_active: self.runs_active.saturating_add(snapshot.active_runs),
            runs_waiting: self.runs_waiting.saturating_add(snapshot.pending_timers),
            runs_failed_total: self
                .runs_failed_total
                .saturating_add(snapshot.counters.runs_failed),
            runs_finished_total: self
                .runs_finished_total
                .saturating_add(snapshot.counters.runs_completed),
            steps_total: self
                .steps_total
                .saturating_add(snapshot.counters.steps_executed),
        }
    }
}

impl Runtime {
    /// Collects runtime metrics from all shards.
    pub fn collect_metrics(&self) -> RuntimeMetricsSnapshot {
        let (shards, totals) = self.shards.iter().enumerate().fold(
            (
                Vec::with_capacity(self.shard_count),
                MetricsTotals::default(),
            ),
            collect_shard_metrics,
        );
        RuntimeMetricsSnapshot {
            shards,
            runs_active: totals.runs_active,
            runs_waiting: totals.runs_waiting,
            runs_failed_total: totals.runs_failed_total,
            runs_finished_total: totals.runs_finished_total,
            steps_total: totals.steps_total,
        }
    }

    /// Returns aggregated counter snapshots from all shards.
    pub fn counters_snapshot(&self) -> CounterSnapshot {
        self.shards
            .iter()
            .map(|shard| shard.counters().snapshot())
            .fold(empty_counter_snapshot(), add_counter_snapshot)
    }

    /// Lists active run summaries across all shards, up to `limit` entries.
    pub fn list_active_runs(
        &self,
        limit: u32,
        workflow_filter: Option<vb_core::WorkflowDigest>,
    ) -> Vec<ActiveRunSummary> {
        let max = limit_to_usize(limit);
        let mut summaries = self
            .shards
            .iter()
            .flat_map(|shard| shard.runs.iter())
            .filter_map(|(run_id, state)| active_run_summary(*run_id, state, workflow_filter))
            .take(max)
            .collect::<Vec<_>>();
        summaries.sort_by_key(|summary| summary.run_id);
        summaries.truncate(max);
        summaries
    }
}

fn collect_shard_metrics(
    (mut snapshots, totals): (Vec<ShardMetricsSnapshot>, MetricsTotals),
    (index, shard): (usize, &Shard),
) -> (Vec<ShardMetricsSnapshot>, MetricsTotals) {
    let snapshot = shard_metrics(index, shard);
    let totals = totals.add_shard(&snapshot);
    snapshots.push(snapshot);
    (snapshots, totals)
}

fn shard_metrics(index: usize, shard: &Shard) -> ShardMetricsSnapshot {
    let counters = shard.counters().snapshot();
    let (frame_pool_free, frame_pool_total) = frame_pool_metrics(shard);
    ShardMetricsSnapshot {
        shard_id: saturating_u32(index),
        active_runs: saturating_u32(shard.active_run_count()),
        command_queue_depth: saturating_u32(shard.command_queue_len()),
        command_queue_remaining: saturating_u32(shard.remaining_capacity()),
        pending_timers: saturating_u32(shard.pending_timer_count()),
        frame_pool_free,
        frame_pool_total,
        trace_ring_fill_pct: trace_fill_pct(shard),
        counters,
    }
}

fn frame_pool_metrics(shard: &Shard) -> (u32, u32) {
    let (free, total) = shard.frame_pool_metrics();
    (saturating_u32(free), saturating_u32(total))
}

fn trace_fill_pct(shard: &Shard) -> f32 {
    let capacity = shard.trace_ring().capacity();
    if capacity == 0 {
        return 0.0;
    }
    let len = shard.trace_ring().len();
    #[allow(clippy::as_conversions)]
    {
        (len as f32) / (capacity as f32) * 100.0
    }
}

fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).map_or(u32::MAX, std::convert::identity)
}

fn empty_counter_snapshot() -> CounterSnapshot {
    CounterSnapshot {
        runs_submitted: 0,
        runs_completed: 0,
        runs_failed: 0,
        steps_executed: 0,
    }
}

fn add_counter_snapshot(total: CounterSnapshot, next: CounterSnapshot) -> CounterSnapshot {
    CounterSnapshot {
        runs_submitted: total.runs_submitted.saturating_add(next.runs_submitted),
        runs_completed: total.runs_completed.saturating_add(next.runs_completed),
        runs_failed: total.runs_failed.saturating_add(next.runs_failed),
        steps_executed: total.steps_executed.saturating_add(next.steps_executed),
    }
}

fn limit_to_usize(limit: u32) -> usize {
    usize::try_from(limit).map_or(usize::MAX, std::convert::identity)
}

fn active_run_summary(
    run_id: RunId,
    state: &RunState,
    workflow_filter: Option<vb_core::WorkflowDigest>,
) -> Option<ActiveRunSummary> {
    let digest = state.workflow.digest();
    if workflow_filter.is_some_and(|filter| digest != filter) {
        return None;
    }
    Some(ActiveRunSummary {
        run_id,
        workflow: digest,
        step_count: state.workflow.node_count(),
        steps_completed: completed_steps(state),
    })
}

fn completed_steps(state: &RunState) -> u16 {
    (0..state.workflow.node_count())
        .map(StepIdx::new)
        .filter(|step| is_completed_step(state, *step))
        .fold(0u16, |count, _| count.saturating_add(1))
}

fn is_completed_step(state: &RunState, step: StepIdx) -> bool {
    matches!(
        state.frame.step_state(step),
        Ok(StepState::Succeeded | StepState::Failed | StepState::Skipped | StepState::Cancelled)
    )
}
