#![forbid(unsafe_code)]
//! Runtime facade shard-control helpers.

use vb_core::action::{ActionFailure, ActionOutputReady, ActionTicket};
use vb_core::ids::RunId;

use crate::runtime::Runtime;
use crate::shard::{ShardCommand, ShardDirective};
use crate::{RuntimeError, RuntimeResult};

/// Summary of an active run on a shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRunSummary {
    /// Run identifier.
    pub run_id: RunId,
    /// Compiled workflow digest.
    pub workflow: vb_core::WorkflowDigest,
    /// Number of workflow steps.
    pub step_count: u16,
    /// Count of steps in a terminal per-step state.
    pub steps_completed: u16,
}

impl Runtime {
    // ── Process control ────────────────────────────────────────────────

    /// Processes one directive on a selected shard.
    pub fn tick_shard(&mut self, shard: u32, directive: ShardDirective) -> RuntimeResult<bool> {
        let source = self.checked_shard_index(shard)?;
        match directive {
            ShardDirective::Continue => self.tick_existing_shard(source),
            ShardDirective::Suspend => self.shard_alive(source),
            ShardDirective::Cancel => Err(RuntimeError::UnsupportedOperation {
                operation: "tick_shard_cancel",
            }),
            ShardDirective::Barrier => Err(RuntimeError::UnsupportedOperation {
                operation: "tick_shard_barrier",
            }),
            ShardDirective::Migrate { target } => self.migrate_selected_shard(source, target),
            ShardDirective::Shutdown => self.shutdown_selected_shard(source),
        }
    }

    /// Processes one command on each shard; false means at least one shard is stopped.
    pub fn tick_all(&mut self) -> RuntimeResult<bool> {
        let mut alive = true;
        for shard in &mut self.shards {
            if !shard.tick()? {
                alive = false;
            }
        }
        Ok(alive)
    }

    // ── Run lifecycle commands ─────────────────────────────────────────

/// Cancels a run.
    pub fn cancel_run(&self, run: RunId) -> RuntimeResult<()> {
        self.cancel_run_with_reason(run, None)
    }

    /// Cancels a run with an optional reason.
    ///
    /// The reason is recorded on the durable `RunCancelled` journal event so
    /// the operator audit trail captures why the run was cancelled. Pass
    /// `None` when no reason is available; the IPC layer uses `None` to
    /// preserve backward compatibility with callers that did not supply a
    /// reason (RQ-W0-11).
    ///
    /// This also serves as the RQ-W0-18 reason-propagation entry point:
    /// the reason is propagated through the queue into the durable
    /// `RunCancelled` journal event so post-mortem operators can attribute
    /// the cancellation.
    pub fn cancel_run_with_reason(
        &self,
        run: RunId,
        reason: Option<String>,
    ) -> RuntimeResult<()> {
        self.shard_for(run)?
            .enqueue(ShardCommand::Cancel { run, reason })
    }

    /// Kills a run.
    pub fn kill_run(&self, run: RunId) -> RuntimeResult<()> {
        self.kill_run_with_reason(run, None)
    }

    /// Kills a run with an optional reason.
    pub fn kill_run_with_reason(&self, run: RunId, reason: Option<String>) -> RuntimeResult<()> {
        self.shard_for(run)?
            .enqueue(ShardCommand::Kill { run, reason })
    }

    /// Resumes a suspended run.
    pub fn resume_run(&self, run: RunId) -> RuntimeResult<()> {
        self.shard_for(run)?.enqueue(ShardCommand::Resume { run })
    }

    // ── Inspection ─────────────────────────────────────────────────────

    /// Enqueues a run inspection command.
    pub fn inspect_run(&self, run: RunId, correlation: u64) -> RuntimeResult<()> {
        self.shard_for(run)?
            .enqueue(ShardCommand::Inspect { run, correlation })
    }

    /// Returns a direct non-queued snapshot from the owning shard.
    pub fn snapshot_run(
        &self,
        run: RunId,
        correlation: u64,
    ) -> RuntimeResult<crate::shard::InspectResponse> {
        Ok(self.shard_for(run)?.snapshot_run(run, correlation))
    }

    /// Takes a pending inspect response from the owning shard.
    pub fn take_inspect_response(
        &mut self,
        run: RunId,
    ) -> RuntimeResult<Option<crate::shard::InspectResponse>> {
        let index = self.shard_index(run);
        let shard = self
            .shards
            .get_mut(index)
            .ok_or(RuntimeError::RunNotFound)?;
        Ok(shard.take_inspect_response())
    }

    /// Lists active run summaries across all shards, up to `limit` entries.
    pub fn list_active_runs(
        &self,
        limit: u32,
        workflow_filter: Option<vb_core::WorkflowDigest>,
    ) -> Vec<ActiveRunSummary> {
        let max = usize::try_from(limit).unwrap_or(usize::MAX);
        let mut summaries = Vec::new();
        for shard in &self.shards {
            collect_shard_summaries(shard, max, workflow_filter, &mut summaries);
            if summaries.len() >= max {
                break;
            }
        }
        summaries.sort_by_key(|summary| summary.run_id);
        summaries.truncate(max);
        summaries
    }

    // ── Action completion ──────────────────────────────────────────────

    /// Completes an action without a typed output payload.
    pub fn complete_action(&self, run: RunId, step: vb_core::ids::StepIdx) -> RuntimeResult<()> {
        self.shard_for(run)?
            .enqueue(ShardCommand::ActionCompletedLegacy { run, step })
    }

    /// Completes an action with a typed output payload.
    pub fn complete_action_with_output(
        &self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(ticket.run)?;
        // Explicitly cancelled or killed runs must reject completions
        // at API time. Naturally-completed runs are accepted here so
        // that IPC re-entry scenarios produce RunNotFound at tick time
        // instead of InvalidActionCompletion at enqueue time.
        if shard.terminal_runs_contains(ticket.run) {
            match shard.terminal_outcome_get(ticket.run) {
                Some(crate::shard::TerminalOutcome::Cancelled)
                | Some(crate::shard::TerminalOutcome::Killed) => {
                    return Err(RuntimeError::InvalidActionCompletion);
                }
                _ => {}
            }
        }
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output })
    }

    /// Fails an action with a typed failure payload.
    pub fn fail_action(&self, ticket: ActionTicket, failure: ActionFailure) -> RuntimeResult<()> {
        let shard = self.shard_for(ticket.run)?;
        if shard.terminal_runs_contains(ticket.run) {
            match shard.terminal_outcome_get(ticket.run) {
                Some(crate::shard::TerminalOutcome::Cancelled)
                | Some(crate::shard::TerminalOutcome::Killed) => {
                    return Err(RuntimeError::InvalidActionCompletion);
                }
                _ => {}
            }
        }
        shard.enqueue(ShardCommand::RuntimeActionFailed { ticket, failure })
    }

    // ── Ask / timer wiring ─────────────────────────────────────────────

    /// Answers an ask with an explicit payload and resume ticket.
    pub fn answer_ask(&self, answer: crate::shard::AskAnswer) -> RuntimeResult<()> {
        let shard = self.shard_for(answer.ticket.run)?;
        if shard.terminal_runs_contains(answer.ticket.run) {
            return Err(RuntimeError::RunNotFound);
        }
        shard.enqueue(ShardCommand::AskAnswered { answer })
    }

    /// Advances a run whose registered wait or ask timer fired externally.
    pub fn timer_fired(&self, run: RunId) -> RuntimeResult<()> {
        let _ = self.shard_for(run)?;
        Err(RuntimeError::InvalidTimerFire)
    }

    /// Captures current timer authority for an externally fired timer.
    pub fn capture_timer_entry(
        &self,
        run: RunId,
    ) -> RuntimeResult<crate::shard::timer_wheel::TimerEntry> {
        self.shard_for(run)?
            .timer_entry(run)
            .ok_or(RuntimeError::InvalidTimerFire)
    }

    /// Advances a timer using captured freshness authority.
    pub fn timer_entry_fired(
        &self,
        entry: crate::shard::timer_wheel::TimerEntry,
    ) -> RuntimeResult<()> {
        self.shard_for(entry.run)?
            .enqueue(ShardCommand::TimerFired {
                run: entry.run,
                generation: entry.generation,
                deadline: entry.deadline,
                kind: entry.kind,
            })
    }

    // ── Shutdown ───────────────────────────────────────────────────────

    /// Shuts down all shards gracefully.
    pub fn shutdown_graceful(&mut self) -> RuntimeResult<()> {
        for shard in &self.shards {
            shard.enqueue(ShardCommand::Shutdown)?;
        }
        for shard in &mut self.shards {
            shard.drain_for_shutdown()?;
        }
        self.journal.drain_for_shutdown()?;
        Ok(())
    }

    // ── Internal helpers ───────────────────────────────────────────────

    fn checked_shard_index(&self, shard: u32) -> RuntimeResult<usize> {
        let index = usize::try_from(shard).map_err(|_| RuntimeError::ShardNotFound { shard })?;
        if index < self.shards.len() {
            Ok(index)
        } else {
            Err(RuntimeError::ShardNotFound { shard })
        }
    }

    fn tick_existing_shard(&mut self, source: usize) -> RuntimeResult<bool> {
        let Some(shard) = self.shards.get_mut(source) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        shard.tick()
    }

    fn shard_alive(&self, source: usize) -> RuntimeResult<bool> {
        let Some(shard) = self.shards.get(source) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        Ok(!shard.is_shutting_down())
    }

    fn migrate_selected_shard(&mut self, source: usize, target: u32) -> RuntimeResult<bool> {
        let target_index = self.checked_shard_index(target)?;
        if source == target_index {
            return Err(RuntimeError::MigrateSelf);
        }
        let commands = self.drain_source_commands(source)?;
        self.enqueue_migrated_commands(target_index, commands)?;
        self.source_has_work(source)
    }

    fn drain_source_commands(&self, source: usize) -> RuntimeResult<Vec<ShardCommand>> {
        let Some(shard) = self.shards.get(source) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        let limit = shard.command_queue_len();
        let mut commands = Vec::with_capacity(limit);
        let mut drained = 0usize;
        while drained < limit {
            let Some(command) = shard.command_queue.pop() else {
                break;
            };
            commands.push(command);
            drained = drained.saturating_add(1);
        }
        Ok(commands)
    }

    fn enqueue_migrated_commands(
        &self,
        target: usize,
        commands: Vec<ShardCommand>,
    ) -> RuntimeResult<()> {
        let Some(shard) = self.shards.get(target) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        for command in commands {
            shard.enqueue(command)?;
        }
        Ok(())
    }

    fn source_has_work(&self, source: usize) -> RuntimeResult<bool> {
        let Some(shard) = self.shards.get(source) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        Ok(shard.active_run_count() > 0 || shard.command_queue_len() > 0)
    }

    fn shutdown_selected_shard(&mut self, source: usize) -> RuntimeResult<bool> {
        let Some(shard) = self.shards.get_mut(source) else {
            return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
        };
        shard.enqueue(ShardCommand::Shutdown)?;
        shard.drain_for_shutdown()?;
        self.journal.drain_for_shutdown()?;
        Ok(false)
    }
}

fn collect_shard_summaries(
    shard: &crate::shard::Shard,
    max: usize,
    workflow_filter: Option<vb_core::WorkflowDigest>,
    summaries: &mut Vec<ActiveRunSummary>,
) {
    let mut index = 0usize;
    while index < shard.runs.len() && summaries.len() < max {
        collect_one_summary(shard, index, workflow_filter, summaries);
        index = index.saturating_add(1);
    }
}

fn collect_one_summary(
    shard: &crate::shard::Shard,
    index: usize,
    workflow_filter: Option<vb_core::WorkflowDigest>,
    summaries: &mut Vec<ActiveRunSummary>,
) {
    let Some((run_id, state)) = shard.runs.get_index(index) else {
        return;
    };
    let digest = state.workflow.digest();
    if workflow_filter.is_some_and(|filter| digest != filter) {
        return;
    }
    summaries.push(ActiveRunSummary {
        run_id: *run_id,
        workflow: digest,
        step_count: state.workflow.node_count(),
        steps_completed: completed_steps(state),
    });
}

fn completed_steps(state: &crate::shard::RunState) -> u16 {
    let mut completed = 0u16;
    let mut step_index = 0u16;
    while step_index < state.workflow.node_count() {
        let step = vb_core::ids::StepIdx::new(step_index);
        if matches!(
            state.frame.step_state(step),
            Ok(vb_core::frame::StepState::Succeeded)
                | Ok(vb_core::frame::StepState::Failed)
                | Ok(vb_core::frame::StepState::Skipped)
                | Ok(vb_core::frame::StepState::Cancelled)
        ) {
            completed = completed.saturating_add(1);
        }
        step_index = step_index.saturating_add(1);
    }
    completed
}
