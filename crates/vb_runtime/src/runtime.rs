#![forbid(unsafe_code)]
//! Multi-shard runtime routing commands to correct shards.

use std::num::NonZeroUsize;
use vb_core::action::{ActionContract, ActionFailure, ActionOutputReady, ActionTicket};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledWorkflow;

use crate::counters::{CounterSnapshot, RuntimeMetricsSnapshot, ShardMetricsSnapshot};
use crate::journal::SharedRuntimeJournal;
use crate::shard::timer_wheel::TimerEntry;
use crate::shard::{AskAnswer, InspectResponse, Shard, ShardCommand, ShardConfig, ShardDirective};
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

/// Summary of an active run on a shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRunSummary {
    /// Run identifier.
    pub run_id: RunId,
    /// Compiled workflow digest.
    pub workflow: vb_core::WorkflowDigest,
    /// Number of steps in the workflow.
    pub step_count: u16,
    /// Steps that reached a terminal state (Succeeded, Failed, Skipped, or Cancelled).
    pub steps_completed: u16,
}

/// Multi-shard runtime.
pub struct Runtime {
    shards: Vec<Shard>,
    shard_count: usize,
    journal: SharedRuntimeJournal,
}

impl Runtime {
    /// Creates a new runtime with the given number of shards and per-shard configuration.
    #[must_use]
    pub fn new(shard_count: NonZeroUsize, config: ShardConfig) -> Self {
        Self::new_with_journal(
            shard_count,
            config,
            crate::journal::VolatileRuntimeJournal::shared(),
        )
    }

    /// Creates a new runtime with an explicit runtime journal sink.
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn new_with_journal(
        shard_count: NonZeroUsize,
        config: ShardConfig,
        journal: SharedRuntimeJournal,
    ) -> Self {
        let count = shard_count.get();
        let mut shards = Vec::with_capacity(count);
        let mut index = 0usize;
        while index < count {
            shards.push(Shard::new_with_journal(config, journal.clone()));
            index = index.saturating_add(1);
        }
        Self {
            shards,
            shard_count: count,
            journal,
        }
    }

    /// Submits a run using a compiled workflow.
    pub fn submit_direct(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        self.submit_direct_with_grants(run, workflow, CapabilitySet::empty())
    }

    /// Submits a run using a compiled workflow and explicit caller grants.
    pub fn submit_direct_with_grants(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission(run, workflow.digest(), caps.clone())?;
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps,
        })
    }

    /// Submits a run with explicit caller grants and validated action contracts.
    pub fn submit_direct_with_grants_and_contracts(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
        action_contracts: Box<[ActionContract]>,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission(run, workflow.digest(), caps.clone())?;
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow,
            caps,
            action_contracts,
        })
    }

    /// Submits a run with pre-mapped input slots, explicit caller grants, and validated action contracts.
    pub fn submit_direct_with_inputs_grants_and_contracts(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
        caps: CapabilitySet,
        action_contracts: Box<[ActionContract]>,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission(run, workflow.digest(), caps.clone())?;
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            inputs,
            caps,
            action_contracts,
        })
    }

    /// Submits a run with inline workflow (same as submit_direct for now).
    pub fn submit_compiled(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        self.submit_direct(run, workflow)
    }

    /// Submits a compiled run with explicit caller grants.
    pub fn submit_compiled_with_grants(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.submit_direct_with_grants(run, workflow, caps)
    }

    /// Submits a run with pre-mapped runtime input slots.
    pub fn submit_compiled_with_inputs(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
    ) -> RuntimeResult<()> {
        self.submit_compiled_with_inputs_and_grants(run, workflow, inputs, CapabilitySet::empty())
    }

    /// Submits a run with pre-mapped runtime input slots and explicit caller grants.
    pub fn submit_compiled_with_inputs_and_grants(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission(run, workflow.digest(), caps.clone())?;
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs,
            caps,
        })
    }

    /// Cancels a run.
    pub fn cancel_run(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Cancel { run, reason: None })
    }

    /// Kills a run unconditionally.
    pub fn kill_run(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Kill { run, reason: None })
    }

    /// Resumes a suspended run from its current program counter.
    pub fn resume_run(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Resume { run })
    }

    /// Inspects run state.
    pub fn inspect_run(&self, run: RunId, correlation: u64) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Inspect { run, correlation })
    }

    /// Returns a direct, non-queued run snapshot from the owning shard.
    pub fn snapshot_run(&self, run: RunId, correlation: u64) -> RuntimeResult<InspectResponse> {
        let shard = self.shard_for(run)?;
        Ok(shard.snapshot_run(run, correlation))
    }

    /// Processes one command on each shard. Returns false if any shard is shutting down.
    pub fn tick_all(&mut self) -> RuntimeResult<bool> {
        let mut alive = true;
        for shard in &mut self.shards {
            if !shard.tick()? {
                alive = false;
            }
        }
        Ok(alive)
    }

    /// Processes one tick on a specific shard with the given directive.
    ///
    /// This method directs a single shard's behavior for one tick. The directive
    /// determines what work the shard performs:
    ///
    /// - `Continue`: Process one command from the queue normally.
    /// - `Suspend`: Skip command processing; preserve queue depth.
    /// - `Migrate { target }`: Transfer all pending commands to the target shard.
    /// - `Shutdown`: Drain all remaining commands and shut down the shard.
    ///
    /// Returns `Ok(true)` if the shard is alive (continuing), `Ok(false)` if the
    /// shard has shut down, or an error if the shard index is invalid or migration
    /// failed.
    pub fn tick_shard(
        &mut self,
        shard_index: u32,
        directive: ShardDirective,
    ) -> RuntimeResult<bool> {
        let shard_index_usize = usize::try_from(shard_index)
            .map_err(|_| RuntimeError::ShardNotFound { shard: shard_index })?;

        // Validate source shard exists first
        if self.shards.get(shard_index_usize).is_none() {
            return Err(RuntimeError::ShardNotFound { shard: shard_index });
        }

        match directive {
            ShardDirective::Continue => {
                let shard = self
                    .shards
                    .get_mut(shard_index_usize)
                    .ok_or(RuntimeError::ShardNotFound { shard: shard_index })?;
                shard.tick()
            }
            ShardDirective::Suspend => {
                // Suspend: skip processing, preserve queue, return alive
                Ok(true)
            }
            ShardDirective::Migrate { target } => self.migrate_shard(shard_index_usize, target),
            ShardDirective::Shutdown => {
                let shard = self
                    .shards
                    .get_mut(shard_index_usize)
                    .ok_or(RuntimeError::ShardNotFound { shard: shard_index })?;
                shard.drain_pending_and_shutdown()?;
                Ok(false)
            }
            ShardDirective::Cancel => Err(RuntimeError::UnsupportedOperation {
                operation: "tick_shard_cancel",
            }),
            ShardDirective::Barrier => Err(RuntimeError::UnsupportedOperation {
                operation: "tick_shard_barrier",
            }),
        }
    }

    /// Migrates all commands from source shard to target shard.
    ///
    /// Returns `Ok(true)` if the source shard is still alive (has runs or pending commands),
    /// `Ok(false)` if the source shard is empty and can be shut down.
    fn migrate_shard(&mut self, source_idx: usize, target: u32) -> RuntimeResult<bool> {
        let target_usize =
            usize::try_from(target).map_err(|_| RuntimeError::ShardNotFound { shard: target })?;

        // Self-migrate check (already validated in caller, but double-check for safety)
        let source_u32 =
            u32::try_from(source_idx).map_err(|_| RuntimeError::ShardNotFound { shard: target })?;
        if target == source_u32 {
            return Err(RuntimeError::MigrateSelf);
        }

        // Validate target shard exists (already validated in caller, but double-check)
        if self.shards.get(target_usize).is_none() {
            return Err(RuntimeError::ShardNotFound { shard: target });
        }

        // Collect all commands from source shard
        let commands: Vec<ShardCommand> = {
            let shard = self
                .shards
                .get_mut(source_idx)
                .ok_or(RuntimeError::ShardNotFound { shard: source_u32 })?;
            let mut cmds = Vec::new();
            while let Some(cmd) = shard.command_queue.pop() {
                cmds.push(cmd);
            }
            cmds
        };

        // Push all commands to target shard
        {
            let target_shard = self
                .shards
                .get_mut(target_usize)
                .ok_or(RuntimeError::ShardNotFound { shard: target })?;
            for cmd in commands {
                target_shard.enqueue(cmd)?;
            }
        }

        // Return true if source shard still has runs (alive)
        let shard = self
            .shards
            .get_mut(source_idx)
            .ok_or(RuntimeError::ShardNotFound { shard: source_u32 })?;
        let alive = shard.active_run_count() > 0 || !shard.command_queue.is_empty();
        Ok(alive)
    }

    /// Completes an action for a run.
    pub fn complete_action(&self, run: RunId, step: StepIdx) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::ActionCompletedLegacy { run, step })
    }

    /// Completes an action for a run with its typed output payload.
    ///
    /// Validates the ticket against the current run state before enqueuing.
    /// Returns `InvalidActionCompletion` if the ticket is invalid (wrong action,
    /// non-running step, or stale attempt).
    pub fn complete_action_with_output(
        &mut self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for_mut(ticket.run)?;
        // Validate ticket before enqueuing — fail fast with InvalidActionCompletion
        // if the ticket doesn't match the current run state.
        if let Some(state) = shard.run_state_get(ticket.run) {
            crate::shard::lifecycle::preflight_action_completion(state, ticket, output)?;
        }
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output })
    }

    /// Fails an action with a typed failure payload.
    pub fn fail_action(&self, ticket: ActionTicket, failure: ActionFailure) -> RuntimeResult<()> {
        let shard = self.shard_for(ticket.run)?;
        shard.enqueue(ShardCommand::RuntimeActionFailed { ticket, failure })
    }

    /// Lists trace events for a run without draining the shard trace ring.
    ///
    /// RA-030 wave-15 (vb-sxkz6): a run may have been migrated to a shard
    /// other than the home shard selected by the hash of `RunId`. Scan all
    /// shards so the trace ring of the owning shard is read.
    pub fn list_events(&self, run: RunId) -> RuntimeResult<Vec<TraceEvent>> {
        let shard = self.shard_for_run(run)?;
        let limit = shard.trace_ring().capacity();
        Ok(shard.trace_ring().snapshot_for_run(run, limit))
    }

    /// Answers an ask with an explicit typed payload and resume ticket.
    ///
    /// RA-030: a run may have been migrated to a shard other than the home
    /// shard selected by the hash of `RunId` (see `migrate_shard` / RA-021).
    /// Test the home shard first as a fast path, then scan the remaining
    /// shards for active or terminal ownership so the answer is routed to
    /// whichever shard actually holds the run state.
    pub fn answer_ask(&self, answer: AskAnswer) -> RuntimeResult<()> {
        let run = answer.ticket.run;
        let shard = self.shard_for_run(run)?;
        shard.enqueue(ShardCommand::AskAnswered { answer })
    }

    /// Legacy run-only timer delivery is fail-closed because it carries no authority.
    pub fn timer_fired(&self, run: RunId) -> RuntimeResult<()> {
        let _shard = self.shard_for_run(run)?;
        Err(RuntimeError::InvalidTimerFire)
    }

    /// Captures the current timer authority for tests and typed scheduler handoff.
    ///
    /// RA-030 wave-15 (vb-sxkz6): scans all shards for the run's timer entry.
    /// Returns `Err(RuntimeError::RunNotFound)` if no shard owns the run.
    /// Returns `Err(RuntimeError::InvalidTimerFire)` when the owner shard has
    /// no live timer entry for the run.
    pub fn capture_timer_entry(&self, run: RunId) -> RuntimeResult<TimerEntry> {
        let shard = self.shard_for_run(run)?;
        shard.timer_entry(run).ok_or(RuntimeError::InvalidTimerFire)
    }

    /// Advances a run from a timer-wheel-captured authority entry.
    ///
    /// RA-030 wave-15 (vb-sxkz6): scans all shards for the run owner.
    pub fn timer_entry_fired(&self, entry: TimerEntry) -> RuntimeResult<()> {
        let shard = self.shard_for_run(entry.run)?;
        shard.enqueue(ShardCommand::TimerFired {
            run: entry.run,
            generation: entry.generation,
            deadline: entry.deadline,
            kind: entry.kind,
        })
    }

    /// Takes the latest inspect response from the run's shard.
    ///
    /// RA-030 wave-15 (vb-sxkz6): scans all shards for the run owner.
    pub fn take_inspect_response(&mut self, run: RunId) -> RuntimeResult<Option<InspectResponse>> {
        let shard = self.shard_for_run_mut(run)?;
        Ok(shard.take_inspect_response())
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
                // SAFETY: trace_len and trace_capacity are bounded by configuration
                // (typically 4096). Safe lossless narrowing to u32 for metric calculation.
                #[allow(clippy::as_conversions)]
                let ratio = (trace_len as f32) / (trace_capacity as f32);
                ratio * 100.0
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
            for (run_id, state) in &shard.runs {
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

    /// Shuts down all shards gracefully.
    pub fn shutdown_graceful(&mut self) -> RuntimeResult<()> {
        for shard in &mut self.shards {
            shard.drain_pending_and_shutdown()?;
        }
        self.journal.drain_for_shutdown()?;
        Ok(())
    }

    fn shard_index(&self, run: RunId) -> usize {
        let hash = run.get();
        let Ok(count) = u64::try_from(self.shard_count) else {
            return 0;
        };
        let Some(remainder) = hash.checked_rem(count) else {
            return 0;
        };
        let Ok(index) = usize::try_from(remainder) else {
            return 0;
        };
        index
    }

    fn shard_for(&self, run: RunId) -> Result<&Shard, RuntimeError> {
        let index = self.shard_index(run);
        self.shards.get(index).ok_or(RuntimeError::RunNotFound)
    }

    fn shard_for_mut(&mut self, run: RunId) -> Result<&mut Shard, RuntimeError> {
        let index = self.shard_index(run);
        self.shards.get_mut(index).ok_or(RuntimeError::RunNotFound)
    }

    /// Returns the shard that currently owns `run`, scanning all shards.
    ///
    /// RA-030 wave-15 (vb-sxkz6): the hash-based `shard_index(run)` returns
    /// the home shard which may differ from the owner after `migrate_shard`
    /// transfers a run's commands and run-state to a different shard. We
    /// check the home shard first as a fast path; on miss we scan the
    /// remaining shards in deterministic slice order.
    ///
    /// Returns `Err(RuntimeError::RunNotFound)` if `run` is not in any
    /// shard's `run_state` or `terminal_runs` set.
    fn shard_for_run(&self, run: RunId) -> Result<&Shard, RuntimeError> {
        let home_index = self.shard_index(run);
        if let Some(shard) = self.shards.get(home_index)
            && (shard.run_state_contains(run) || shard.terminal_runs_contains(run))
        {
            return Ok(shard);
        }
        self.shards
            .iter()
            .enumerate()
            .find(|(idx, shard)| {
                *idx != home_index
                    && (shard.run_state_contains(run) || shard.terminal_runs_contains(run))
            })
            .map(|(_, shard)| shard)
            .ok_or(RuntimeError::RunNotFound)
    }

    /// Mutable variant of `shard_for_run` for `take_inspect_response`.
    fn shard_for_run_mut(&mut self, run: RunId) -> Result<&mut Shard, RuntimeError> {
        // Mirror the home-fast-path + scan-fallback logic on immutable borrow,
        // then re-index after borrow ends to get the mutable shard.
        let home_index = self.shard_index(run);
        let owner_index = {
            if let Some(shard) = self.shards.get(home_index)
                && (shard.run_state_contains(run) || shard.terminal_runs_contains(run))
            {
                home_index
            } else {
                self.shards
                    .iter()
                    .position(|shard| {
                        shard.run_state_contains(run) || shard.terminal_runs_contains(run)
                    })
                    .ok_or(RuntimeError::RunNotFound)?
            }
        };
        self.shards
            .get_mut(owner_index)
            .ok_or(RuntimeError::RunNotFound)
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
