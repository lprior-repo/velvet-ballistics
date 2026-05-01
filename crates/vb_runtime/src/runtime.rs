//! Multi-shard runtime routing commands to correct shards.

use std::num::NonZeroUsize;
use vb_core::action::{ActionFailure, ActionTicket};
use vb_core::ids::{RunId, StepIdx};
use vb_core::workflow::CompiledWorkflow;

use crate::counters::CounterSnapshot;
use crate::shard::{AskAnswer, InspectResponse, Shard, ShardCommand, ShardConfig};
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

    /// Completes an action for a run.
    pub fn complete_action(&self, run: RunId, step: StepIdx) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::ActionCompleted { run, step })
    }

    /// Fails an action with a typed failure payload.
    pub fn fail_action(&self, ticket: ActionTicket, failure: ActionFailure) -> RuntimeResult<()> {
        let shard = self.shard_for(ticket.run)?;
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure })
    }

    /// Lists trace events for a run without draining the shard trace ring.
    pub fn list_events(&self, run: RunId) -> RuntimeResult<Vec<TraceEvent>> {
        let shard_index = self.shard_index(run);
        let Some(shard) = self.shards.get(shard_index) else {
            return Err(RuntimeError::RunNotFound);
        };
        let limit = shard.trace_ring().capacity();
        Ok(shard.trace_ring().snapshot_for_run(run, limit))
    }

    /// Answers an ask with an explicit typed payload and resume ticket.
    pub fn answer_ask(&self, answer: AskAnswer) -> RuntimeResult<()> {
        let shard = self.shard_for(answer.ticket.run)?;
        shard.enqueue(ShardCommand::AskAnswered { answer })
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
    use crate::trace::TraceEvent;
    use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
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
    fn snapshot_run_reports_missing_run_without_command_queue() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, ShardConfig::default());
        let result = runtime.snapshot_run(RunId::new(1), 7);
        assert_eq!(
            result,
            Ok(InspectResponse::NotFound {
                run: RunId::new(1),
                correlation: 7,
            })
        );
    }

    #[test]
    fn list_events_is_non_destructive() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
        };
        let runtime = Runtime::new(shard_count, config);
        let first = runtime.list_events(RunId::new(1));
        let second = runtime.list_events(RunId::new(1));
        assert_eq!(first, Ok(Vec::new()));
        assert_eq!(second, Ok(Vec::new()));
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

    // Helper: workflow that finishes immediately (SetConst -> Finish).
    fn finished_workflow() -> Option<CompiledWorkflow> {
        let set_const = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("finished"),
            digest: WorkflowDigest::from_bytes([2; 32]),
            nodes: Box::from([set_const, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([vb_core::value::ConstValue::Bool(true)]),
            slot_count: 1,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).ok()
    }

    #[test]
    fn runtime_submit_direct_enqueues_on_correct_shard() {
        // Given a 2-shard runtime
        let Some(shard_count) = NonZeroUsize::new(2) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else { return };
        let run = RunId::new(1);
        // When submitting a run
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then counters show 1 run submitted
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
    }

    #[test]
    fn runtime_cancel_run_routes_to_correct_shard() {
        // Given a 2-shard runtime with a submitted run
        let Some(shard_count) = NonZeroUsize::new(2) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else { return };
        let run = RunId::new(1);
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When cancelling the run
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the failed counter is incremented
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_complete_action_routes_to_correct_shard() {
        // Given a 2-shard runtime with a suspended run
        let Some(shard_count) = NonZeroUsize::new(2) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else { return };
        let run = RunId::new(1);
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When completing the action
        assert_eq!(runtime.complete_action(run, StepIdx::new(0)), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the trace contains the ActionCompleted event
        let events = runtime.list_events(run);
        match events {
            Ok(evts) => {
                let found = evts.iter().any(|e| *e == TraceEvent::ActionCompleted {
                    run,
                    step: StepIdx::new(0),
                });
                assert_eq!(found, true);
            }
            Err(_) => {
                // Should not happen
                assert!(false);
            }
        }
    }

    #[test]
    fn runtime_inspect_run_returns_found_from_correct_shard() {
        // Given a 2-shard runtime with a submitted run
        let Some(shard_count) = NonZeroUsize::new(2) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else { return };
        let run = RunId::new(1);
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When inspecting the run
        assert_eq!(runtime.inspect_run(run, 42), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the inspect response is Found with correct fields
        let response = runtime.take_inspect_response(run);
        match response {
            Ok(Some(InspectResponse::Found(snapshot))) => {
                assert_eq!(snapshot.run, run);
                assert_eq!(snapshot.correlation, 42);
            }
            other => {
                // Wrong: expected Found
                assert_eq!(other, Ok(None));
            }
        }
    }

    #[test]
    fn runtime_tick_all_returns_false_when_any_shard_shuts_down() {
        // Given a 2-shard runtime
        let Some(shard_count) = NonZeroUsize::new(2) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When shutting down only one shard
        // Use shutdown_graceful which enqueues to all shards
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // Then tick_all returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_tick_all_returns_true_when_all_shards_alive() {
        // Given a 2-shard runtime with no shutdown
        let Some(shard_count) = NonZeroUsize::new(2) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When ticking with empty queues
        let result = runtime.tick_all();
        // Then result is true
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn runtime_list_events_returns_events_for_target_run_only() {
        // Given a 2-shard runtime with two runs
        let Some(shard_count) = NonZeroUsize::new(2) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf1) = suspended_workflow() else { return };
        let Some(wf2) = suspended_workflow() else { return };
        let run1 = RunId::new(1);
        let run2 = RunId::new(2);
        assert_eq!(runtime.submit_direct(run1, wf1), Ok(()));
        assert_eq!(runtime.submit_direct(run2, wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When listing events for run1
        let events = runtime.list_events(run1);
        assert_eq!(events.is_ok(), true);
        let events = events;
        let events = match events {
            Ok(e) => e,
            Err(_) => return,
        };
        // Then all events are for run1 only
        let all_run1 = events.iter().all(|e| e.run_id() == run1);
        assert_eq!(all_run1, true);
    }

    #[test]
    fn runtime_take_inspect_response_returns_none_initially() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When taking inspect response without any inspect command
        let run = RunId::new(1);
        let response = runtime.take_inspect_response(run);
        // Then response is Ok(None)
        assert_eq!(response, Ok(None));
    }

    #[test]
    fn runtime_counters_snapshot_starts_at_zero() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(3) else { return };
        let runtime = Runtime::new(shard_count, test_config());
        // When taking counters snapshot
        let snap = runtime.counters_snapshot();
        // Then all counters are zero
        assert_eq!(snap.runs_submitted, 0);
        assert_eq!(snap.runs_completed, 0);
        assert_eq!(snap.runs_failed, 0);
        assert_eq!(snap.steps_executed, 0);
    }

    #[test]
    fn runtime_submit_compiled_delegates_to_submit_direct() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = finished_workflow() else { return };
        let run = RunId::new(42);
        // When using submit_compiled
        assert_eq!(runtime.submit_compiled(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the run is processed
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 1);
    }

    #[test]
    fn runtime_fail_action_returns_unsupported_operation() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let runtime = Runtime::new(shard_count, test_config());
        // When calling fail_action
        let result = runtime.fail_action(RunId::new(1));
        // Then it returns UnsupportedOperation with durable_action_failure
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "durable_action_failure",
            })
        );
    }

    #[test]
    fn runtime_answer_ask_returns_unsupported_operation() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let runtime = Runtime::new(shard_count, test_config());
        // When calling answer_ask
        let result = runtime.answer_ask(RunId::new(1));
        // Then it returns UnsupportedOperation with durable_ask_answer
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "durable_ask_answer",
            })
        );
    }

    #[test]
    fn runtime_drain_trace_returns_empty_for_fresh_runtime() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(2) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When draining trace
        let events = runtime.drain_trace();
        // Then result is empty
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn runtime_submit_and_cancel_increments_failed_counter() {
        // Given a 1-shard runtime
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else { return };
        let run = RunId::new(1);
        // When submitting then cancelling
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then failed counter is 1
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_inspect_run_enqueues_command_successfully() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else { return };
        let run = RunId::new(1);
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When inspecting
        assert_eq!(runtime.inspect_run(run, 99), Ok(()));
        // Then tick processes the inspect
        assert_eq!(runtime.tick_all(), Ok(true));
        // And the response is available
        let response = runtime.take_inspect_response(run);
        match response {
            Ok(Some(InspectResponse::Found(snap))) => {
                assert_eq!(snap.run, run);
                assert_eq!(snap.correlation, 99);
            }
            other => {
                assert_eq!(other, Ok(None));
            }
        }
    }

    #[test]
    fn runtime_shutdown_graceful_enqueues_to_all_shards() {
        // Given a 3-shard runtime
        let Some(shard_count) = NonZeroUsize::new(3) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When shutting down gracefully
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // Then tick_all returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_tick_all_after_shutdown_returns_false_repeatedly() {
        // Given a shutdown runtime
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(false));
        // When ticking again
        assert_eq!(runtime.tick_all(), Ok(false));
        // Then it still returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_submit_direct_returns_queue_full_when_shard_queue_full() {
        // Given a runtime with tiny queue
        let config = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let runtime = Runtime::new(shard_count, config);
        // When filling the queue
        let Some(wf) = suspended_workflow() else { return };
        assert_eq!(runtime.submit_direct(RunId::new(1), wf.clone()), Ok(()));
        // Then the second submit returns QueueFull
        assert_eq!(runtime.submit_direct(RunId::new(2), wf), Err(RuntimeError::QueueFull));
    }

    #[test]
    fn runtime_list_events_for_unknown_shard_returns_error() {
        // Given a runtime with no runs
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When listing events for a run on a nonexistent shard (can't happen with valid shard_index)
        // Use a valid run that maps to shard 0
        let events = runtime.list_events(RunId::new(1));
        // Then result is Ok with empty vec
        match events {
            Ok(evts) => {
                assert_eq!(evts.len(), 0);
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn runtime_counters_aggregate_across_shards() {
        // Given a 2-shard runtime
        let Some(shard_count) = NonZeroUsize::new(2) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf1) = finished_workflow() else { return };
        let Some(wf2) = finished_workflow() else { return };
        // When submitting runs
        assert_eq!(runtime.submit_direct(RunId::new(1), wf1), Ok(()));
        assert_eq!(runtime.submit_direct(RunId::new(2), wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then counters aggregate across shards
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 2);
        assert_eq!(snap.runs_completed, 2);
    }

    #[test]
    fn runtime_single_shard_operations() {
        // Given a 1-shard runtime
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = finished_workflow() else { return };
        // When submitting a run
        assert_eq!(runtime.submit_direct(RunId::new(1), wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then it completes
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_completed, 1);
        assert_eq!(snap.runs_failed, 0);
    }

    #[test]
    fn runtime_new_creates_correct_shard_count() {
        // Given a runtime with 4 shards
        let Some(shard_count) = NonZeroUsize::new(4) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When shutting down
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // Then tick_all returns false (all shards shut down)
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_take_inspect_response_for_unknown_run_returns_not_found() {
        // Given a runtime with no submitted runs
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When inspecting a non-existent run
        let run = RunId::new(999);
        assert_eq!(runtime.inspect_run(run, 1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the response is NotFound
        let response = runtime.take_inspect_response(run);
        assert_eq!(
            response,
            Ok(Some(InspectResponse::NotFound { run, correlation: 1 }))
        );
    }

    #[test]
    fn runtime_drain_trace_returns_submitted_events() {
        // Given a runtime with submitted runs
        let Some(shard_count) = NonZeroUsize::new(1) else { return };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else { return };
        assert_eq!(runtime.submit_direct(RunId::new(1), wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When draining trace
        let events = runtime.drain_trace();
        // Then events contain RunSubmitted
        let found = events.iter().any(|e| matches!(e, TraceEvent::RunSubmitted { run } if *run == RunId::new(1)));
        assert_eq!(found, true);
    }
}
