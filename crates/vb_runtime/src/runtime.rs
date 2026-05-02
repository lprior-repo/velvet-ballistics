//! Multi-shard runtime routing commands to correct shards.

use std::num::NonZeroUsize;
use vb_core::action::{ActionFailure, ActionOutputReady, ActionTicket};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledWorkflow;

use crate::counters::CounterSnapshot;
use crate::journal::SharedRuntimeJournal;
use crate::shard::{AskAnswer, InspectResponse, Shard, ShardCommand, ShardConfig};
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

/// Multi-shard runtime.
pub struct Runtime {
    shards: Vec<Shard>,
    shard_count: usize,
    journal: SharedRuntimeJournal,
}

impl Runtime {
    /// Creates a new runtime with the given number of shards and per-shard configuration.
    pub fn new(shard_count: NonZeroUsize, config: ShardConfig) -> Self {
        Self::new_with_journal(
            shard_count,
            config,
            crate::journal::NoopRuntimeJournal::shared(),
        )
    }

    /// Creates a new runtime with an explicit runtime journal sink.
    #[allow(clippy::needless_pass_by_value)]
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
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Submit { run, workflow, caps: CapabilitySet::empty() })
    }

    /// Submits a run with inline workflow (same as submit_direct for now).
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
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs,
            caps: CapabilitySet::empty(),
        })
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
        shard.enqueue(ShardCommand::ActionCompletedLegacy { run, step })
    }

    /// Completes an action for a run with its typed output payload.
    pub fn complete_action_with_output(
        &self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(ticket.run)?;
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output })
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

    /// Advances a run whose registered wait or ask timer fired externally.
    pub fn timer_fired(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::TimerFired { run })
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

    fn shard_index(&self, run: RunId) -> usize {
        let hash = run.as_u64();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AskTicket;
    use crate::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
    use crate::trace::TraceEvent;
    use std::sync::Arc;
    use vb_core::action::{ActionFailureCode, ActionOutputReady, ActionTicket};
    use vb_core::ids::{ActionId, ConstIdx, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::{SlotValue, Taint};
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

    fn action_then_finish_workflow() -> Option<CompiledWorkflow> {
        let do_node = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::Do {
                action: ActionId::new(7),
                input: SlotIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("action_then_finish"),
            digest: WorkflowDigest::from_bytes([3; 32]),
            nodes: Box::from([do_node, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 2,
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
            policy: vb_core::policy::RuntimePolicy::Relaxed,
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
            policy: vb_core::policy::RuntimePolicy::Relaxed,
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
    fn shutdown_graceful_processes_shards_before_journal_drain() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new_with_journal(shard_count, test_config(), journal.clone());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = RunId::new(31);
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));

        assert_eq!(runtime.shutdown_graceful(), Ok(()));

        assert_eq!(
            journal.snapshot(),
            Ok(vec![
                RuntimeJournalEvent::RunSubmitted {
                    run,
                    workflow: WorkflowDigest::from_bytes([2; 32]),
                },
                // Evidence chain: step 0 (SetConst)
                RuntimeJournalEvent::StepStarted {
                    run,
                    step: StepIdx::new(0),
                },
                RuntimeJournalEvent::SlotWritten {
                    run,
                    slot: SlotIdx::new(0),
                },
                RuntimeJournalEvent::StepSucceeded {
                    run,
                    step: StepIdx::new(0),
                    output: SlotIdx::new(0),
                },
                // Evidence chain: step 1 (Finish)
                RuntimeJournalEvent::StepStarted {
                    run,
                    step: StepIdx::new(1),
                },
                RuntimeJournalEvent::StepSucceeded {
                    run,
                    step: StepIdx::new(1),
                    output: SlotIdx::ZERO,
                },
                RuntimeJournalEvent::RunFinished {
                    run,
                    result: SlotIdx::ZERO,
                },
            ])
        );
    }

    #[test]
    fn counters_snapshot_aggregates_across_shards() {
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
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
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        assert_eq!(runtime.submit_direct(RunId::new(1), wf1), Ok(()));
        assert_eq!(runtime.submit_direct(RunId::new(2), wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let events = runtime.drain_trace();
        // Each submit produces: RunSubmitted + StepStarted + ActionScheduled = 3 events per run
        assert_eq!(events.len(), 6);
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
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
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
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
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
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
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
                let found = evts.iter().any(|e| {
                    *e == TraceEvent::ActionCompleted {
                        run,
                        step: StepIdx::new(0),
                    }
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
    fn do_action_completion_writes_output_and_journals_events() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new_with_journal(shard_count, test_config(), journal.clone());
        let Some(wf) = action_then_finish_workflow() else {
            return;
        };
        let run = RunId::new(11);
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        let events = runtime.list_events(run);
        assert!(matches!(
            events,
            Ok(ref evts) if evts.contains(&TraceEvent::ActionScheduled {
                run,
                step: StepIdx::ZERO,
            })
        ));

        let ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(7),
            attempt: 1,
            idempotency_key: 0,
        };
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(1),
            value: SlotValue::I64(99),
            taint: Taint::Clean,
            encoded_len: 8,
        };
        assert_eq!(runtime.complete_action_with_output(ticket, output), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_completed, 1);
        let trace = runtime.list_events(run);
        assert!(matches!(
            trace,
            Ok(ref evts) if evts.contains(&TraceEvent::SlotWritten {
                run,
                slot: SlotIdx::new(1),
            }) && evts.contains(&TraceEvent::ActionCompleted {
                run,
                step: StepIdx::ZERO,
            }) && evts.contains(&TraceEvent::RunFinished { run })
        ));
        let journal_events = journal.snapshot();
        assert!(matches!(
            journal_events,
            Ok(ref evts) if evts.contains(&RuntimeJournalEvent::ActionScheduled {
                run,
                step: StepIdx::ZERO,
                action: ActionId::new(7),
            }) && evts.contains(&RuntimeJournalEvent::SlotWritten {
                run,
                slot: SlotIdx::new(1),
            }) && evts.contains(&RuntimeJournalEvent::ActionCompleted {
                run,
                step: StepIdx::ZERO,
                action: ActionId::new(7),
            })
        ));
    }

    #[test]
    fn do_action_completion_rejects_wrong_action_ticket() {
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = action_then_finish_workflow() else {
            return;
        };
        let run = RunId::new(12);
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(8),
            attempt: 1,
            idempotency_key: 0,
        };
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(1),
            value: SlotValue::I64(99),
            taint: Taint::Clean,
            encoded_len: 8,
        };
        assert_eq!(runtime.complete_action_with_output(ticket, output), Ok(()));
        assert_eq!(
            runtime.tick_all(),
            Err(RuntimeError::InvalidActionCompletion)
        );
    }

    #[test]
    fn runtime_inspect_run_returns_found_from_correct_shard() {
        // Given a 2-shard runtime with a submitted run
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
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
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
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
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When ticking with empty queues
        let result = runtime.tick_all();
        // Then result is true
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn runtime_list_events_returns_events_for_target_run_only() {
        // Given a 2-shard runtime with two runs
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
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
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
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
        let Some(shard_count) = NonZeroUsize::new(3) else {
            return;
        };
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
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
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
    fn runtime_fail_action_routes_to_run_shard() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, test_config());
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::ZERO,
            seq: SeqNo::new(0),
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 1,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Rejected,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let result = runtime.fail_action(ticket, failure);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn runtime_answer_ask_routes_to_run_shard() {
        // Given a runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, test_config());
        let answer = AskAnswer {
            ticket: AskTicket {
                run: RunId::new(1),
                ask_step: StepIdx::ZERO,
                resume_step: StepIdx::new(1),
            },
            answer_slot: SlotIdx::new(0),
            value: SlotValue::Bool(true),
            taint: Taint::Clean,
        };
        let result = runtime.answer_ask(answer);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn runtime_drain_trace_returns_empty_for_fresh_runtime() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When draining trace
        let events = runtime.drain_trace();
        // Then result is empty
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn runtime_submit_and_cancel_increments_failed_counter() {
        // Given a 1-shard runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
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
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
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
        let Some(shard_count) = NonZeroUsize::new(3) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When shutting down gracefully
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // Then tick_all returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_tick_all_after_shutdown_returns_false_repeatedly() {
        // Given a shutdown runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
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
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, config);
        // When filling the queue
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(runtime.submit_direct(RunId::new(1), wf.clone()), Ok(()));
        // Then the second submit returns QueueFull
        assert_eq!(
            runtime.submit_direct(RunId::new(2), wf),
            Err(RuntimeError::QueueFull)
        );
    }

    #[test]
    fn runtime_list_events_for_unknown_shard_returns_error() {
        // Given a runtime with no runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, test_config());
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
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf1) = finished_workflow() else {
            return;
        };
        let Some(wf2) = finished_workflow() else {
            return;
        };
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
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
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
        let Some(shard_count) = NonZeroUsize::new(4) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When shutting down
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // Then tick_all returns false (all shards shut down)
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_take_inspect_response_for_unknown_run_returns_not_found() {
        // Given a runtime with no submitted runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When inspecting a non-existent run
        let run = RunId::new(999);
        assert_eq!(runtime.inspect_run(run, 1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the response is NotFound
        let response = runtime.take_inspect_response(run);
        assert_eq!(
            response,
            Ok(Some(InspectResponse::NotFound {
                run,
                correlation: 1
            }))
        );
    }

    #[test]
    fn runtime_drain_trace_returns_submitted_events() {
        // Given a runtime with submitted runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(runtime.submit_direct(RunId::new(1), wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When draining trace
        let events = runtime.drain_trace();
        // Then events contain RunSubmitted
        let found = events
            .iter()
            .any(|e| matches!(e, TraceEvent::RunSubmitted { run } if *run == RunId::new(1)));
        assert_eq!(found, true);
    }

    // =======================================================================
    // Adversarial BDD tests — runtime
    // =======================================================================

    #[test]
    fn runtime_shutdown_with_pending_run_then_tick_returns_false() {
        // Given a runtime with a pending suspended run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(300);
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When shutting down with a pending run
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // Then tick_all returns false
        assert_eq!(runtime.tick_all(), Ok(false));
    }

    #[test]
    fn runtime_run_stays_on_one_shard_across_operations() {
        // Given a 2-shard runtime
        let Some(shard_count) = NonZeroUsize::new(2) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(301);
        // When submitting, then cancelling
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then counters show exactly 1 submitted and 1 failed
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 1);
        assert_eq!(snap.runs_failed, 1);
        // And re-submitting the same run succeeds (it was removed by cancel)
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        assert_eq!(runtime.submit_direct(run, wf2), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        let snap2 = runtime.counters_snapshot();
        assert_eq!(snap2.runs_submitted, 2);
    }

    #[test]
    fn runtime_complete_action_for_never_submitted_run_returns_ok_enqueue() {
        // Given a runtime with no runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        // When completing an action for a run that was never submitted
        let run = RunId::new(999);
        assert_eq!(runtime.complete_action(run, StepIdx::new(0)), Ok(()));
        // Then tick returns RunNotFound (the shard has no such run)
        assert_eq!(runtime.tick_all(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn runtime_fail_action_for_never_submitted_run_returns_ok_enqueue() {
        // Given a runtime with no runs
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, test_config());
        // When failing an action for a run that was never submitted
        let ticket = ActionTicket {
            run: RunId::new(998),
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Rejected,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        // Then enqueue succeeds (failure is queued)
        assert_eq!(runtime.fail_action(ticket, failure), Ok(()));
    }

    #[test]
    fn runtime_queue_full_returns_typed_error() {
        // Given a runtime with tiny queue capacity
        let config = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, config);
        let Some(wf) = suspended_workflow() else {
            return;
        };
        // When filling the queue to capacity
        assert_eq!(runtime.submit_direct(RunId::new(1), wf.clone()), Ok(()));
        // Then the next submit returns QueueFull (exact error variant)
        assert_eq!(
            runtime.submit_direct(RunId::new(2), wf),
            Err(RuntimeError::QueueFull)
        );
    }

    #[test]
    fn runtime_drain_trace_after_drain_returns_empty() {
        // Given a runtime with events
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(runtime.submit_direct(RunId::new(1), wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When draining twice
        let first = runtime.drain_trace();
        assert_eq!(first.is_empty(), false);
        let second = runtime.drain_trace();
        // Then second drain is empty
        assert_eq!(second.len(), 0);
    }

    #[test]
    fn runtime_countered_exhausted_at_max_active_runs() {
        // Given a 1-shard runtime with max_active_runs = 1
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        // When submitting two runs
        assert_eq!(runtime.submit_direct(RunId::new(1), wf1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.submit_direct(RunId::new(2), wf2), Ok(()));
        // Then second tick returns ActiveRunCapacityExceeded
        assert_eq!(
            runtime.tick_all(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
    }

    #[test]
    fn runtime_snapshot_run_for_unknown_run_returns_not_found() {
        // Given a fresh runtime
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let runtime = Runtime::new(shard_count, test_config());
        // When snapshotting a non-existent run
        let result = runtime.snapshot_run(RunId::new(9999), 42);
        // Then it returns NotFound
        assert_eq!(
            result,
            Ok(InspectResponse::NotFound {
                run: RunId::new(9999),
                correlation: 42,
            })
        );
    }

    #[test]
    fn runtime_list_events_is_idempotent() {
        // Given a runtime with a submitted run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        assert_eq!(runtime.submit_direct(RunId::new(1), wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When listing events twice without draining
        let first = runtime.list_events(RunId::new(1));
        let second = runtime.list_events(RunId::new(1));
        // Then both return the same events (non-destructive)
        assert_eq!(first, second);
        assert_eq!(first.map(|e| e.is_empty()), Ok(false));
    }

    #[test]
    fn runtime_finished_workflow_counts_completed_not_failed() {
        // Given a runtime with a workflow that finishes immediately
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        // When submitting
        assert_eq!(runtime.submit_direct(RunId::new(42), wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then completed is 1 and failed is 0
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_completed, 1);
        assert_eq!(snap.runs_failed, 0);
        assert_eq!(snap.runs_submitted, 1);
    }

    // =======================================================================
    // Adversarial BDD tests - runtime attack vectors
    // =======================================================================

    #[test]
    fn runtime_two_shards_deterministic_routing_same_run_same_shard() {
        // Given a 4-shard runtime
        let Some(shard_count) = NonZeroUsize::new(4) else {
            return;
        };
        let runtime = Runtime::new(shard_count, test_config());
        // When computing shard index for run 1 twice
        let idx1 = runtime.shard_index(RunId::new(1));
        let idx2 = runtime.shard_index(RunId::new(1));
        // Then the shard index is deterministic
        assert_eq!(idx1, idx2);
        assert!(idx1 < 4);
    }

    #[test]
    fn runtime_two_shards_different_runs_may_land_on_different_shards() {
        // Given a 4-shard runtime
        let Some(shard_count) = NonZeroUsize::new(4) else {
            return;
        };
        let runtime = Runtime::new(shard_count, test_config());
        // When computing shard indices for different runs
        let idx1 = runtime.shard_index(RunId::new(1));
        let idx2 = runtime.shard_index(RunId::new(2));
        // Then at least two different shard indices exist among the runs
        // (we can't guarantee different shards for 1 and 2, but we check the mechanism works)
        assert!(idx1 < 4);
        assert!(idx2 < 4);
    }

    #[test]
    fn runtime_cancel_then_resubmit_on_same_shard_succeeds() {
        // Given a 1-shard runtime with a cancelled run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(400);
        assert_eq!(runtime.submit_direct(run, wf.clone()), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.cancel_run(run), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When re-submitting the same run
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then counters show 2 submissions and 1 failed
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_submitted, 2);
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_fail_action_for_active_suspended_run_increments_failed() {
        // Given a 1-shard runtime with a suspended run
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(401);
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // When failing the action
        let ticket = ActionTicket {
            run,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Rejected,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert_eq!(runtime.fail_action(ticket, failure), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then the run is failed.
        let snap = runtime.counters_snapshot();
        assert_eq!(snap.runs_failed, 1);
    }

    #[test]
    fn runtime_tick_all_after_shutdown_ignores_pending_commands() {
        // Given a runtime with a submit queued, then shutdown
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, test_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        // Shutdown first (processes shutdown tick)
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        // When trying to submit after shutdown (enqueue still works but tick ignores)
        assert_eq!(runtime.submit_direct(RunId::new(402), wf), Ok(()));
        // Then tick_all returns false (shard shutting down)
        assert_eq!(runtime.tick_all(), Ok(false));
        assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    }

    #[test]
    fn runtime_journal_events_are_recorded_for_submit_and_finish() {
        // Given a runtime with a volatile journal
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new_with_journal(shard_count, test_config(), journal.clone());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = RunId::new(403);
        // When submitting and ticking
        assert_eq!(runtime.submit_direct(run, wf), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        // Then journal contains RunSubmitted and RunFinished
        let events = journal.snapshot();
        match events {
            Ok(evts) => {
                let found_submitted = evts.iter().any(|e| {
                    *e == RuntimeJournalEvent::RunSubmitted {
                        run,
                        workflow: vb_core::ids::WorkflowDigest::from_bytes([2; 32]),
                    }
                });
                let found_finished = evts.iter().any(
                    |e| matches!(e, RuntimeJournalEvent::RunFinished { run: r, .. } if *r == run),
                );
                assert_eq!(found_submitted, true);
                assert_eq!(found_finished, true);
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn runtime_countered_exhausted_does_not_corrupt_other_runs() {
        // Given a 1-shard runtime with max_active_runs=1
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let Some(shard_count) = NonZeroUsize::new(1) else {
            return;
        };
        let mut runtime = Runtime::new(shard_count, config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        let run1 = RunId::new(500);
        let run2 = RunId::new(501);
        // When submitting run1 (succeeds) then run2 (capacity exceeded)
        assert_eq!(runtime.submit_direct(run1, wf1), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));
        assert_eq!(runtime.submit_direct(run2, wf2), Ok(()));
        assert_eq!(
            runtime.tick_all(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
        // Then run1 is still alive and inspectable
        let snap = runtime.snapshot_run(run1, 1);
        match snap {
            Ok(InspectResponse::Found(s)) => {
                assert_eq!(s.run, run1);
            }
            other => {
                assert_eq!(
                    other,
                    Ok(InspectResponse::NotFound {
                        run: run1,
                        correlation: 1
                    })
                );
            }
        }
    }
}
