#![forbid(unsafe_code)]
//! Multi-shard runtime facade routing public commands to owning shards.

use std::num::NonZeroUsize;

use vb_core::action::{ActionFailure, ActionOutputReady, ActionTicket};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledWorkflow;

use crate::counters::{CounterSnapshot, RuntimeMetricsSnapshot};
use crate::journal::SharedRuntimeJournal;
use crate::shard::{AskAnswer, InspectResponse, Shard, ShardCommand, ShardConfig};
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

#[path = "runtime_metrics.rs"]
mod runtime_metrics;
use runtime_metrics::collect_metrics;
#[path = "runtime_control.rs"]
mod runtime_control;
pub use runtime_control::ActiveRunSummary;

/// Multi-shard runtime facade.
pub struct Runtime {
    pub(crate) shards: Vec<Shard>,
    shard_count: usize,
    journal: SharedRuntimeJournal,
}

impl Runtime {
    /// Creates a runtime with a noop journal sink.
    #[must_use]
    pub fn new(shard_count: NonZeroUsize, config: ShardConfig) -> Self {
        Self::new_with_journal(
            shard_count,
            config,
            crate::journal::NoopRuntimeJournal::shared(),
        )
    }

    /// Creates a runtime with an explicit runtime journal sink.
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
        self.shard_for(run)?.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: CapabilitySet::empty(),
        })
    }

    /// Submits a run using a compiled workflow.
    pub fn submit_compiled(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        self.submit_direct(run, workflow)
    }

    /// Submits a run with pre-mapped runtime input slots.
    pub fn submit_compiled_with_inputs(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
    ) -> RuntimeResult<()> {
        self.shard_for(run)?
            .enqueue(ShardCommand::SubmitWithInputs {
                run,
                workflow,
                inputs,
                caps: CapabilitySet::empty(),
            })
    }

    /// Submits a run with inputs, grants, and prevalidated action contracts.
    pub fn submit_direct_with_inputs_grants_and_contracts(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
        caps: CapabilitySet,
        action_contracts: Box<[vb_core::action::ActionContract]>,
    ) -> RuntimeResult<()> {
        self.shard_for(run)?
            .enqueue(ShardCommand::SubmitWithInputsAndContracts {
                run,
                workflow,
                inputs,
                caps,
                action_contracts,
            })
    }

    /// Cancels a run.
    pub fn cancel_run(&self, run: RunId) -> RuntimeResult<()> {
        self.shard_for(run)?
            .enqueue(ShardCommand::Cancel { run, reason: None })
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

    /// Enqueues a run inspection command.
    pub fn inspect_run(&self, run: RunId, correlation: u64) -> RuntimeResult<()> {
        self.shard_for(run)?
            .enqueue(ShardCommand::Inspect { run, correlation })
    }

    /// Returns a direct non-queued snapshot from the owning shard.
    pub fn snapshot_run(&self, run: RunId, correlation: u64) -> RuntimeResult<InspectResponse> {
        Ok(self.shard_for(run)?.snapshot_run(run, correlation))
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

    /// Completes an action without a typed output payload.
    pub fn complete_action(&self, run: RunId, step: StepIdx) -> RuntimeResult<()> {
        self.shard_for(run)?
            .enqueue(ShardCommand::ActionCompletedLegacy { run, step })
    }

    /// Completes an action with a typed output payload.
    pub fn complete_action_with_output(
        &self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        self.shard_for(ticket.run)?
            .enqueue(ShardCommand::ActionCompleted { ticket, output })
    }

    /// Fails an action with a typed failure payload.
    pub fn fail_action(&self, ticket: ActionTicket, failure: ActionFailure) -> RuntimeResult<()> {
        self.shard_for(ticket.run)?
            .enqueue(ShardCommand::ActionFailed { ticket, failure })
    }

    /// Lists trace events for one run without draining.
    pub fn list_events(&self, run: RunId) -> RuntimeResult<Vec<TraceEvent>> {
        let shard = self.shard_for(run)?;
        let limit = shard.trace_ring().capacity();
        Ok(shard.trace_ring().snapshot_for_run(run, limit))
    }

    /// Answers an ask with an explicit payload and resume ticket.
    pub fn answer_ask(&self, answer: AskAnswer) -> RuntimeResult<()> {
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

    /// Takes the latest inspect response from the run's shard.
    pub fn take_inspect_response(&mut self, run: RunId) -> RuntimeResult<Option<InspectResponse>> {
        let index = self.shard_index(run);
        let shard = self
            .shards
            .get_mut(index)
            .ok_or(RuntimeError::RunNotFound)?;
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
        collect_metrics(&self.shards, self.shard_count)
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

    /// Computes the owning shard index for a run.
    #[must_use]
    pub fn shard_index(&self, run: RunId) -> usize {
        let Ok(count) = u64::try_from(self.shard_count) else {
            return 0;
        };
        let Some(remainder) = run.get().checked_rem(count) else {
            return 0;
        };
        usize::try_from(remainder).map_or(0, core::convert::identity)
    }

    fn shard_for(&self, run: RunId) -> Result<&Shard, RuntimeError> {
        self.shards
            .get(self.shard_index(run))
            .ok_or(RuntimeError::RunNotFound)
    }
}
