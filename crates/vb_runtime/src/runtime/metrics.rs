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
            let counters = shard.counters().snapshot();
            let active_runs = u32::try_from(shard.active_run_count()).unwrap_or(u32::MAX);
            let queue_depth = u32::try_from(shard.command_queue_len()).unwrap_or(u32::MAX);
            let queue_remaining = u32::try_from(shard.remaining_capacity()).unwrap_or(u32::MAX);
            let pending_timers = u32::try_from(shard.pending_timer_count()).unwrap_or(u32::MAX);
            let (fp_free, fp_total) = shard.frame_pool_metrics();
            let frame_pool_free = u32::try_from(fp_free).unwrap_or(u32::MAX);
            let frame_pool_total = u32::try_from(fp_total).unwrap_or(u32::MAX);
            let trace_capacity = shard.trace_ring().capacity();
            let trace_len = shard.trace_ring().pending_len();
            let trace_ring_fill_pct = if trace_capacity > 0 {
                // Bounded narrowing mirrors the six sibling metric lines at runtime.rs:571-577.
                // TraceRing::new clamps capacity to >= 1 (RA-003 cap bound 2^20 << 2^24), so the
                // unwrap_or(0) fallback is unreachable in production. Fallback value is 0 (not
                // u32::MAX) to preserve the sentinel intent of the outer zero-denominator guard.
                //
                // DEVIATION FROM CONTRACT INV-004: `f32::from(u32)` is NOT implemented in
                // Rust (only `From<u8|u16|i8|i16>` exist for f32). See `u32_to_f32_exact`
                // above for the bit-equivalent IEEE-754 manual encoding; equivalence to
                // `(n as f32)` is verified in `.beads/vb-oul6u/evidence/ieee-754-bit-equivalence.log`
                // and pinned by the RA-003 corpus (`trace_ring_fill_pct` tests, 3/3 pass).
                let cap_u32 = u32::try_from(trace_capacity).unwrap_or(0);
                let len_u32 = u32::try_from(trace_len).unwrap_or(0);
                let len_f32 = u32_to_f32_exact(len_u32);
                let cap_f32 = u32_to_f32_exact(cap_u32);
                (len_f32 / cap_f32) * 100.0
            } else {
                0.0
            };

            runs_active = runs_active.saturating_add(active_runs);
            runs_waiting = runs_waiting.saturating_add(pending_timers);
            runs_failed_total = runs_failed_total.saturating_add(counters.runs_failed);
            runs_finished_total = runs_finished_total.saturating_add(counters.runs_completed);
            steps_total = steps_total.saturating_add(counters.steps_executed);

            let shard_id = u32::try_from(index).unwrap_or(u32::MAX);
            shards.push(ShardMetricsSnapshot {
                shard_id,
                active_runs,
                command_queue_depth: queue_depth,
                command_queue_remaining: queue_remaining,
                pending_timers,
                frame_pool_free,
                frame_pool_total,
                trace_ring_fill_pct,
                counters,
            });
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
                let digest = state.workflow.digest();
                if let Some(filter) = workflow_filter
                    && digest != filter
                {
                    continue;
                }
                let step_count = state.workflow.node_count();
                let mut steps_completed: u16 = 0;
                let mut step_index = 0u16;
                while step_index < step_count {
                    let step = vb_core::ids::StepIdx::new(step_index);
                    match state.frame.step_state(step) {
                        Ok(
                            vb_core::frame::StepState::Succeeded
                            | vb_core::frame::StepState::Failed
                            | vb_core::frame::StepState::Skipped
                            | vb_core::frame::StepState::Cancelled,
                        ) => {
                            steps_completed = steps_completed.saturating_add(1);
                        }
                        Ok(_) => {}
                        Err(_) => {
                            step_index = step_index.saturating_add(1);
                            continue;
                        }
                    }
                    step_index = step_index.saturating_add(1);
                }
                summaries.push(ActiveRunSummary {
                    run_id: *run_id,
                    workflow: digest,
                    step_count,
                    steps_completed,
                });
            }
        }
        summaries.sort_by_key(|s| s.run_id);
        summaries.truncate(max);
        summaries
    }
}
