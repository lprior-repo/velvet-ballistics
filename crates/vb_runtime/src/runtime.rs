//! Multi-shard runtime routing commands to correct shards.

use std::num::NonZeroUsize;
use vb_core::ids::{RunId, StepIdx};
use vb_core::workflow::CompiledWorkflow;

use crate::counters::CounterSnapshot;
use crate::shard::{Shard, ShardCommand, ShardConfig};
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

/// Multi-shard runtime.
pub struct Runtime {
    shards: Vec<Shard>,
    shard_count: usize,
}

impl Runtime {
    /// Creates a new runtime with the given number of shards and per-shard configuration.
    pub fn new(shard_count: NonZeroUsize, config: ShardConfig) -> Self {
        let count = shard_count.get();
        let shards = (0..count).map(|_| Shard::new(config.clone())).collect();
        Self {
            shards,
            shard_count: count,
        }
    }

    /// Submits a run using a compiled workflow.
    pub fn submit_direct(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Submit { run, workflow })
    }

    /// Submits a run with inline workflow (same as submit_direct for now).
    pub fn submit_compiled(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
    ) -> RuntimeResult<()> {
        self.submit_direct(run, workflow)
    }

    /// Cancels a run.
    pub fn cancel_run(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Cancel { run })
    }

    /// Inspects run state.
    pub fn inspect_run(&self, run: RunId, correlation: u64) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Inspect { run, correlation })
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

    /// Completes an action for a run.
    pub fn complete_action(&self, run: RunId, step: StepIdx) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::ActionCompleted { run, step })
    }

    /// Fails an action (treated as a cancellation).
    pub fn fail_action(&self, run: RunId) -> RuntimeResult<()> {
        self.cancel_run(run)
    }

    /// Lists events for a run by draining the shard's trace ring.
    pub fn list_events(&mut self, run: RunId) -> Vec<TraceEvent> {
        let shard_index = self.shard_index(run);
        let Some(shard) = self.shards.get_mut(shard_index) else {
            return Vec::new();
        };
        let all = shard.trace_ring_mut().drain();
        filter_run_events(all, run)
    }

    /// Answers an ask by resuming the run.
    pub fn answer_ask(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Resume { run })
    }

    /// Drains all trace events from all shards.
    pub fn drain_trace(&mut self) -> Vec<TraceEvent> {
        let mut events = Vec::new();
        for shard in &mut self.shards {
            events.extend(shard.trace_ring_mut().drain());
        }
        events
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
    pub fn shutdown_graceful(&self) -> RuntimeResult<()> {
        for shard in &self.shards {
            shard.enqueue(ShardCommand::Shutdown)?;
        }
        Ok(())
    }

    #[allow(clippy::arithmetic_side_effects)]
    fn shard_index(&self, run: RunId) -> usize {
        let hash = run.as_u64();
        let count = u64::try_from(self.shard_count).unwrap_or(1);
        let remainder = hash % count;
        usize::try_from(remainder).unwrap_or(0)
    }

    fn shard_for(&self, run: RunId) -> Result<&Shard, RuntimeError> {
        let index = self.shard_index(run);
        self.shards.get(index).ok_or(RuntimeError::RunNotFound)
    }
}

fn filter_run_events(events: Vec<TraceEvent>, target: RunId) -> Vec<TraceEvent> {
    events
        .into_iter()
        .filter(|event| match event {
            TraceEvent::RunSubmitted { run }
            | TraceEvent::RunFinished { run }
            | TraceEvent::RunFailed { run }
            | TraceEvent::StepStarted { run, .. }
            | TraceEvent::StepEnded { run, .. }
            | TraceEvent::SlotWritten { run, .. }
            | TraceEvent::ActionScheduled { run, .. }
            | TraceEvent::ActionCompleted { run, .. } => *run == target,
        })
        .collect()
}
