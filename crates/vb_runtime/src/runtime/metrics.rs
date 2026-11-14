impl Runtime {
    /// Collects runtime metrics from all shards.
    pub fn collect_metrics(&self) -> RuntimeMetricsSnapshot {
        let mut shards = Vec::with_capacity(self.shard_count);
        let mut runs_active = 0u32;
        let mut runs_waiting = 0u32;
        let mut runs_failed_total = 0u64;
        let mut runs_finished_total = 0u64;
        let mut steps_total = 0u64;

        for (index, shard) in self.shards.iter().enumerate() {
            let shard_id = u32::try_from(index).unwrap_or(u32::MAX);
            let snapshot = shard_metrics_snapshot(shard_id, shard);
            runs_active = runs_active.saturating_add(snapshot.active_runs);
            runs_waiting = runs_waiting.saturating_add(snapshot.pending_timers);
            runs_failed_total = runs_failed_total.saturating_add(snapshot.counters.runs_failed);
            runs_finished_total =
                runs_finished_total.saturating_add(snapshot.counters.runs_completed);
            steps_total = steps_total.saturating_add(snapshot.counters.steps_executed);
            shards.push(snapshot);
        }

        RuntimeMetricsSnapshot {
            shards,
            runs_active,
            runs_waiting,
            runs_failed_total,
            runs_finished_total,
            steps_total,
        }
    }

    /// Returns aggregated counter snapshots from all shards.
    pub fn counters_snapshot(&self) -> CounterSnapshot {
        let mut total = CounterSnapshot {
            runs_submitted: 0,
            runs_completed: 0,
            runs_failed: 0,
            steps_executed: 0,
        };
        for shard in &self.shards {
            let snap = shard.counters().snapshot();
            total.runs_submitted = total.runs_submitted.saturating_add(snap.runs_submitted);
            total.runs_completed = total.runs_completed.saturating_add(snap.runs_completed);
            total.runs_failed = total.runs_failed.saturating_add(snap.runs_failed);
            total.steps_executed = total.steps_executed.saturating_add(snap.steps_executed);
        }
        total
    }

    /// Lists active run summaries across all shards, up to `limit` entries.
    ///
    /// If `workflow_filter` is provided, only runs matching that digest are included.
    /// Returns summaries sorted by run id ascending.
    pub fn list_active_runs(
        &self,
        limit: u32,
        workflow_filter: Option<vb_core::WorkflowDigest>,
    ) -> Vec<ActiveRunSummary> {
        let max = match usize::try_from(limit) {
            Ok(value) => value,
            Err(_) => usize::MAX,
        };
        let mut summaries = Vec::new();
        for shard in &self.shards {
            for (run_id, state) in shard.active_runs_iter() {
                if summaries.len() >= max {
                    break;
                }
                if let Some(summary) = active_run_summary(*run_id, state, workflow_filter) {
                    summaries.push(summary);
                }
            }
        }
        summaries.sort_by_key(|s| s.run_id);
        summaries.truncate(max);
        summaries
    }
}

fn active_run_summary(
    run_id: RunId,
    state: &crate::shard::types::RunState,
    workflow_filter: Option<vb_core::WorkflowDigest>,
) -> Option<ActiveRunSummary> {
    let digest = state.workflow.digest();
    if workflow_filter.is_some_and(|filter| digest != filter) {
        return None;
    }
    let step_count = state.workflow.node_count();
    Some(ActiveRunSummary {
        run_id,
        workflow: digest,
        step_count,
        steps_completed: completed_step_count(&state.frame, step_count),
    })
}

fn completed_step_count(frame: &vb_core::frame::RunFrame, step_count: u16) -> u16 {
    let mut completed = 0u16;
    let mut step_index = 0u16;
    while step_index < step_count {
        let step = vb_core::ids::StepIdx::new(step_index);
        if let Ok(
            vb_core::frame::StepState::Succeeded
            | vb_core::frame::StepState::Failed
            | vb_core::frame::StepState::Skipped
            | vb_core::frame::StepState::Cancelled,
        ) = frame.step_state(step)
        {
            completed = completed.saturating_add(1);
        }
        step_index = step_index.saturating_add(1);
    }
    completed
}

fn shard_metrics_snapshot(shard_id: u32, shard: &crate::shard::Shard) -> ShardMetricsSnapshot {
    let counters = shard.counters().snapshot();
    let active_runs = u32::try_from(shard.active_run_count()).unwrap_or(u32::MAX);
    let command_queue_depth = u32::try_from(shard.command_queue_len()).unwrap_or(u32::MAX);
    let command_queue_remaining = u32::try_from(shard.remaining_capacity()).unwrap_or(u32::MAX);
    let pending_timers = u32::try_from(shard.pending_timer_count()).unwrap_or(u32::MAX);
    let (free, total) = shard.frame_pool_metrics();
    let frame_pool_free = u32::try_from(free).unwrap_or(u32::MAX);
    let frame_pool_total = u32::try_from(total).unwrap_or(u32::MAX);
    let trace_ring_fill_pct = trace_fill_percent(
        shard.trace_ring().capacity(),
        shard.trace_ring().pending_len(),
    );
    ShardMetricsSnapshot {
        shard_id,
        active_runs,
        command_queue_depth,
        command_queue_remaining,
        pending_timers,
        frame_pool_free,
        frame_pool_total,
        trace_ring_fill_pct,
        counters,
    }
}

fn trace_fill_percent(trace_capacity: usize, trace_len: usize) -> f32 {
    if trace_capacity == 0 {
        return 0.0;
    }
    let cap_u32 = u32::try_from(trace_capacity).unwrap_or(0);
    let len_u32 = u32::try_from(trace_len).unwrap_or(0);
    (u32_to_f32_exact(len_u32) / u32_to_f32_exact(cap_u32)) * 100.0
}
