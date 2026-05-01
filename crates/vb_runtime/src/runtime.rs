//! Multi-shard runtime routing commands to correct shards.

use std::num::NonZeroUsize;
use vb_core::ids::{RunId, StepIdx};
use vb_core::workflow::CompiledWorkflow;

use crate::counters::CounterSnapshot;
use crate::shard::{InspectResponse, Shard, ShardCommand, ShardConfig};
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
    pub fn submit_direct(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Submit { run, workflow })
    }

    /// Submits a run with inline workflow (same as submit_direct for now).
    pub fn submit_compiled(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
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

    /// Fails an action. Durable failure routing is not implemented yet.
    pub fn fail_action(&self, run: RunId) -> RuntimeResult<()> {
        let _shard = self.shard_for(run)?;
        Err(RuntimeError::UnsupportedOperation {
            operation: "durable_action_failure",
        })
    }

    /// Lists trace events for a run by bounded-draining the shard trace ring.
    pub fn list_events(&mut self, run: RunId) -> RuntimeResult<Vec<TraceEvent>> {
        let shard_index = self.shard_index(run);
        let Some(shard) = self.shards.get_mut(shard_index) else {
            return Err(RuntimeError::RunNotFound);
        };
        let limit = shard.trace_ring_mut().capacity();
        Ok(shard.trace_ring_mut().drain_for_run(run, limit))
    }

    /// Answers an ask. Durable answer injection is not implemented yet.
    pub fn answer_ask(&self, run: RunId) -> RuntimeResult<()> {
        let _shard = self.shard_for(run)?;
        Err(RuntimeError::UnsupportedOperation {
            operation: "durable_ask_answer",
        })
    }

    /// Takes the latest inspect response from the run's shard.
    pub fn take_inspect_response(&mut self, run: RunId) -> RuntimeResult<Option<InspectResponse>> {
        let shard_index = self.shard_index(run);
        let shard = self
            .shards
            .get_mut(shard_index)
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

    fn shard_index(&self, run: RunId) -> usize {
        let hash = run.as_u64();
        let count = match u64::try_from(self.shard_count) {
            Ok(value) => value,
            Err(_) => return 0,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    fn suspended_workflow() -> Option<CompiledWorkflow> {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("suspended"),
            digest: WorkflowDigest::from_bytes([1; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn test_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
        }
    }

    #[test]
    fn ask_answer_reports_unsupported_durable_path() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, ShardConfig::default());
        let result = runtime.answer_ask(RunId::new(1));
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "durable_ask_answer",
            })
        );
    }

    #[test]
    fn fail_action_reports_unsupported_durable_path() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, ShardConfig::default());
        let result = runtime.fail_action(RunId::new(1));
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "durable_action_failure",
            })
        );
    }

    #[test]
    fn new_creates_configured_shard_count() {
        let Some(shard_count) = NonZeroUsize::new(3) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn shutdown_graceful_enqueues_on_all_shards() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn counters_snapshot_aggregates_across_shards() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf1) = suspended_workflow() else { return };
        let Some(wf2) = suspended_workflow() else { return };
        assert_eq!(runtime.submit_direct(RunId::new(1), wf1), Ok(()));
        assert_eq!(runtime.submit_direct(RunId::new(2), wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 2);
    }

    #[test]
    fn drain_trace_aggregates_across_shards() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf1) = suspended_workflow() else { return };
        let Some(wf2) = suspended_workflow() else { return };
        assert_eq!(runtime.submit_direct(RunId::new(1), wf1), Ok(()));
        assert_eq!(runtime.submit_direct(RunId::new(2), wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let events = runtime.drain_trace();
        // Each submit produces RunSubmitted + ActionScheduled = 2 events per run
        assert_eq!(events.len(), 4);
    }
}
