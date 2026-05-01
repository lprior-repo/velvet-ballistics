//! Single-threaded shard owning mutable run state directly.

use crossbeam_queue::ArrayQueue;
use std::collections::HashMap;
use vb_core::action::{ActionFailure, ActionTicket};
use vb_core::engine::{StepBudget, new_run_frame};
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::counters::ShardCounters;
use crate::engine::{RetryPolicy, RuntimeEngineResult, RuntimeSignal, drive_deterministic_full};
use crate::trace::{TraceEvent, TraceRing};
use crate::{RuntimeError, RuntimeResult};

/// Bounded command processed by a shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShardCommand {
    /// Submit a new run for execution.
    Submit {
        /// Run identifier chosen by the caller.
        run: RunId,
        /// Compiled workflow to execute.
        workflow: CompiledWorkflow,
    },
    /// Resume a suspended run from its current program counter.
    Resume {
        /// Run identifier.
        run: RunId,
    },
    /// An external action completed.
    ActionCompleted {
        /// Run identifier.
        run: RunId,
        /// Step that was waiting for this action.
        step: StepIdx,
    },
    /// An external action failed.
    ActionFailed {
        /// Ticket for the action being failed.
        ticket: ActionTicket,
        /// Typed failure payload.
        failure: ActionFailure,
    },
    /// An external ask was answered.
    AskAnswered {
        /// Typed ask answer payload.
        answer: AskAnswer,
    },
    /// A timer fired for a suspended run.
    TimerFired {
        /// Run identifier.
        run: RunId,
    },
    /// Cancel an active run.
    Cancel {
        /// Run identifier.
        run: RunId,
    },
    /// Inspect run state for diagnostic purposes.
    Inspect {
        /// Run identifier.
        run: RunId,
        /// Caller correlation identifier echoed in the response.
        correlation: u64,
    },
    /// Shut down the shard gracefully.
    Shutdown,
}

/// Ticket identifying where an ask answer must resume execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AskTicket {
    /// Owning run.
    pub run: RunId,
    /// Step that issued the ask and is currently marked asking.
    pub ask_step: StepIdx,
    /// Step that consumes the answer slot, usually an AskResume node.
    pub resume_step: StepIdx,
}

/// Explicit ask answer contract. The caller supplies both payload and destination slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AskAnswer {
    /// Ask ticket proving the intended resume point.
    pub ticket: AskTicket,
    /// Slot that receives the answer before resuming.
    pub answer_slot: vb_core::ids::SlotIdx,
    /// Answer payload.
    pub value: SlotValue,
    /// Answer taint marker.
    pub taint: Taint,
}

/// Mutable run state owned directly by the shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunState {
    /// Active run frame.
    pub frame: RunFrame,
    /// Compiled workflow for this run.
    pub workflow: CompiledWorkflow,
    /// Cold value store for list, object, and blob handles.
    pub store: ValueStore,
}

/// Diagnostic snapshot returned by the Inspect command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectSnapshot {
    /// Run identifier.
    pub run: RunId,
    /// Caller correlation identifier.
    pub correlation: u64,
    /// Current program counter.
    pub pc: StepIdx,
    /// Number of executed transitions.
    pub executed: u64,
}

/// Bounded response produced by an inspect command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectResponse {
    /// The run was active and a snapshot was captured.
    Found(InspectSnapshot),
    /// The run was not active on this shard.
    NotFound {
        /// Run identifier.
        run: RunId,
        /// Caller correlation identifier.
        correlation: u64,
    },
}

/// Single-threaded shard owning all mutable run state.
pub struct Shard {
    command_queue: ArrayQueue<ShardCommand>,
    runs: HashMap<RunId, RunState>,
    trace_ring: TraceRing,
    counters: ShardCounters,
    step_budget_per_tick: u64,
    max_active_runs: usize,
    inspect_response: Option<InspectResponse>,
    shutting_down: bool,
}

impl Shard {
    /// Creates a new shard with the given configuration.
    pub fn new(config: ShardConfig) -> Self {
        Self {
            command_queue: ArrayQueue::new(config.command_queue_capacity),
            runs: HashMap::new(),
            trace_ring: TraceRing::new(config.trace_capacity),
            counters: ShardCounters::new(),
            step_budget_per_tick: config.step_budget_per_tick,
            max_active_runs: config.max_active_runs,
            inspect_response: None,
            shutting_down: false,
        }
    }

    /// Enqueues a command. Returns `QueueFull` on overflow.
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        self.command_queue
            .push(cmd)
            .map_err(|_| RuntimeError::QueueFull)
    }

    /// Processes one command from the queue. Returns false if the shard should shut down.
    pub fn tick(&mut self) -> RuntimeResult<bool> {
        if self.shutting_down {
            return Ok(false);
        }

        let cmd = match self.command_queue.pop() {
            Some(cmd) => cmd,
            None => return Ok(true),
        };

        match cmd {
            ShardCommand::Submit { run, workflow } => self.handle_submit(run, workflow)?,
            ShardCommand::Resume { run } => self.handle_resume(run)?,
            ShardCommand::ActionCompleted { run, step } => {
                self.handle_action_completion(run, step)?
            }
            ShardCommand::ActionFailed { ticket, failure } => {
                self.handle_action_failure(ticket, failure)?;
            }
            ShardCommand::AskAnswered { answer } => self.handle_ask_answer(answer)?,
            ShardCommand::TimerFired { run } => self.handle_timer(run)?,
            ShardCommand::Cancel { run } => self.handle_cancel(run)?,
            ShardCommand::Inspect { run, correlation } => {
                self.handle_inspect(run, correlation);
            }
            ShardCommand::Shutdown => {
                self.shutting_down = true;
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Returns a reference to the shard counters.
    #[must_use]
    pub const fn counters(&self) -> &ShardCounters {
        &self.counters
    }

    /// Returns a mutable reference to the trace ring.
    pub fn trace_ring_mut(&mut self) -> &mut TraceRing {
        &mut self.trace_ring
    }

    /// Returns an immutable reference to the trace ring.
    #[must_use]
    pub const fn trace_ring(&self) -> &TraceRing {
        &self.trace_ring
    }

    /// Returns a direct non-queued diagnostic snapshot for a run.
    #[must_use]
    pub fn snapshot_run(&self, run: RunId, correlation: u64) -> InspectResponse {
        match self.runs.get(&run) {
            Some(state) => InspectResponse::Found(snapshot_from_state(run, correlation, state)),
            None => InspectResponse::NotFound { run, correlation },
        }
    }

    /// Takes the latest inspect response, if one is available.
    pub fn take_inspect_response(&mut self) -> Option<InspectResponse> {
        self.inspect_response.take()
    }

    /// Returns true if the shard is shutting down.
    #[must_use]
    pub const fn is_shutting_down(&self) -> bool {
        self.shutting_down
    }

    fn handle_submit(&mut self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        if self.runs.contains_key(&run) {
            return Err(RuntimeError::RunAlreadyExists);
        }
        if self.runs.len() >= self.max_active_runs {
            return Err(RuntimeError::ActiveRunCapacityExceeded {
                capacity: self.max_active_runs,
            });
        }
        let frame = new_run_frame(run, &workflow).map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring.push(TraceEvent::RunSubmitted { run });
        self.counters.inc_submitted();
        let state = RunState {
            frame,
            workflow,
            store: ValueStore::new(),
        };
        self.runs.insert(run, state);
        self.drive_run(run)?;
        Ok(())
    }

    fn handle_resume(&mut self, run: RunId) -> RuntimeResult<()> {
        self.drive_run(run)
    }

    fn handle_action_completion(&mut self, run: RunId, step: StepIdx) -> RuntimeResult<()> {
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring
            .push(TraceEvent::ActionCompleted { run, step });
        self.drive_run(run)
    }

    fn handle_action_failure(
        &mut self,
        ticket: ActionTicket,
        failure: ActionFailure,
    ) -> RuntimeResult<()> {
        let run = ticket.run;
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_failed(ticket.step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring.push(TraceEvent::ActionFailed {
            run,
            step: ticket.step,
            code: failure.code,
        });
        self.fail_run(run);
        Ok(())
    }

    fn handle_ask_answer(&mut self, answer: AskAnswer) -> RuntimeResult<()> {
        let run = answer.ticket.run;
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .write_slot_with_taint(answer.answer_slot, answer.value, answer.taint)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(answer.ticket.ask_step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state.frame.set_pc(answer.ticket.resume_step);
        self.trace_ring.push(TraceEvent::AskAnswered {
            run,
            step: answer.ticket.ask_step,
            slot: answer.answer_slot,
        });
        self.drive_run(run)
    }

    fn handle_timer(&mut self, run: RunId) -> RuntimeResult<()> {
        self.drive_run(run)
    }

    fn handle_cancel(&mut self, run: RunId) -> RuntimeResult<()> {
        if self.runs.remove(&run).is_some() {
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunFailed { run });
        }
        Ok(())
    }

    fn handle_inspect(&mut self, run: RunId, correlation: u64) {
        self.inspect_response = Some(self.snapshot_run(run, correlation));
    }

    fn drive_run(&mut self, run: RunId) -> RuntimeResult<()> {
        let mut state = self.take_run_state(run)?;
        let result = Self::drive_state(&mut state, self.step_budget_per_tick);
        self.apply_drive_result(run, state, result);
        Ok(())
    }

    fn take_run_state(&mut self, run: RunId) -> RuntimeResult<RunState> {
        match self.runs.remove(&run) {
            Some(state) => Ok(state),
            None => Err(RuntimeError::RunNotFound),
        }
    }

    fn drive_state(
        state: &mut RunState,
        step_budget_per_tick: u64,
    ) -> RuntimeEngineResult<RuntimeSignal> {
        let mut budget = StepBudget::new(step_budget_per_tick);
        drive_deterministic_full(
            &state.workflow,
            &mut state.frame,
            &mut budget,
            &mut state.store,
            &[],
            RetryPolicy::NEVER,
        )
    }

    fn apply_drive_result(
        &mut self,
        run: RunId,
        state: RunState,
        result: RuntimeEngineResult<RuntimeSignal>,
    ) {
        match result {
            Ok(RuntimeSignal::Continue) => self.keep_run(run, state),
            Ok(RuntimeSignal::Finished(_)) => self.finish_run(run, state),
            Ok(RuntimeSignal::StepBudgetExhausted) => self.keep_run(run, state),
            Ok(RuntimeSignal::AwaitingAction(_)) => self.await_action(run, state),
            Ok(RuntimeSignal::AwaitingWait) => self.keep_run(run, state),
            Ok(RuntimeSignal::AwaitingAsk) => self.keep_run(run, state),
            Err(_) => self.fail_run(run),
        }
    }

    fn keep_run(&mut self, run: RunId, state: RunState) {
        self.counters.add_steps(state.frame.executed());
        self.runs.insert(run, state);
    }

    fn finish_run(&mut self, run: RunId, state: RunState) {
        self.counters.inc_completed();
        self.counters.add_steps(state.frame.executed());
        self.trace_ring.push(TraceEvent::RunFinished { run });
    }

    fn await_action(&mut self, run: RunId, state: RunState) {
        self.counters.add_steps(state.frame.executed());
        let step = state.frame.pc();
        self.trace_ring
            .push(TraceEvent::ActionScheduled { run, step });
        self.runs.insert(run, state);
    }

    fn fail_run(&mut self, run: RunId) {
        self.counters.inc_failed();
        self.trace_ring.push(TraceEvent::RunFailed { run });
    }
}

fn snapshot_from_state(run: RunId, correlation: u64, state: &RunState) -> InspectSnapshot {
    InspectSnapshot {
        run,
        correlation,
        pc: state.frame.pc(),
        executed: state.frame.executed(),
    }
}

/// Shard configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardConfig {
    /// Bounded capacity for the command queue.
    pub command_queue_capacity: usize,
    /// Bounded capacity for the trace ring.
    pub trace_capacity: usize,
    /// Maximum steps to execute per tick.
    pub step_budget_per_tick: u64,
    /// Maximum active runs admitted to this shard.
    pub max_active_runs: usize,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            command_queue_capacity: 1024,
            trace_capacity: 4096,
            step_budget_per_tick: 1000,
            max_active_runs: 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, ConstIdx, SlotIdx, WorkflowDigest};
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

    #[test]
    fn shard_rejects_active_run_capacity_overflow() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
        };
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };

        let first = shard.enqueue(ShardCommand::Submit {
            run: RunId::new(1),
            workflow: workflow.clone(),
        });
        assert_eq!(first, Ok(()));
        assert_eq!(shard.tick(), Ok(true));

        let second = shard.enqueue(ShardCommand::Submit {
            run: RunId::new(2),
            workflow,
        });
        assert_eq!(second, Ok(()));
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
    }

    #[test]
    fn inspect_command_stores_retrievable_snapshot() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
        };
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(7);

        let submitted = shard.enqueue(ShardCommand::Submit { run, workflow });
        assert_eq!(submitted, Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        let inspected = shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 99,
        });
        assert_eq!(inspected, Ok(()));
        assert_eq!(shard.tick(), Ok(true));

        match shard.take_inspect_response() {
            Some(InspectResponse::Found(snapshot)) => {
                assert_eq!(snapshot.run, run);
                assert_eq!(snapshot.correlation, 99);
            }
            other => assert_eq!(other, None),
        }
    }

    #[test]
    fn enqueue_shutdown_sets_shutting_down_flag() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
        };
        let mut shard = Shard::new(config);
        assert_eq!(shard.is_shutting_down(), false);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.is_shutting_down(), true);
    }

    #[test]
    fn tick_returns_true_when_queue_is_empty() {
        let config = ShardConfig::default();
        let mut shard = Shard::new(config);
        assert_eq!(shard.tick(), Ok(true));
    }

    #[test]
    fn cancel_nonexistent_run_succeeds_silently() {
        let config = ShardConfig::default();
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.enqueue(ShardCommand::Cancel { run: RunId::new(999) }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    #[test]
    fn counters_reflect_submitted_after_submit_tick() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
        };
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(1);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit { run, workflow }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    }

    #[test]
    fn inspect_nonexistent_run_returns_not_found() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 1,
        };
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run: RunId::new(999),
                correlation: 42,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.take_inspect_response(),
            Some(InspectResponse::NotFound {
                run: RunId::new(999),
                correlation: 42,
            })
        );
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

    fn small_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
        }
    }

    #[test]
    fn enqueue_returns_queue_full_when_capacity_exceeded() {
        // Given a shard with very small command queue
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
        };
        let shard = Shard::new(config);
        // When enqueuing more commands than capacity allows
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        // Then the third enqueue returns QueueFull
        assert_eq!(
            shard.enqueue(ShardCommand::Shutdown),
            Err(RuntimeError::QueueFull)
        );
    }

    #[test]
    fn tick_after_shutdown_returns_false() {
        // Given a shard that has received a shutdown command
        let config = small_config();
        let mut shard = Shard::new(config);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        // When ticking after shutdown
        assert_eq!(shard.tick(), Ok(false));
        // Then subsequent tick also returns false (shutting_down flag is set)
        assert_eq!(shard.tick(), Ok(false));
    }

    #[test]
    fn submit_returns_run_already_exists_for_duplicate() {
        // Given a shard with an active run
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(42);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: workflow.clone(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // When submitting the same run ID again
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        // Then tick returns RunAlreadyExists
        assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
    }

    #[test]
    fn submit_returns_active_run_capacity_exceeded_at_limit() {
        // Given a shard with max_active_runs = 1 and one active run
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 1,
        };
        let mut shard = Shard::new(config);
        let Some(wf) = suspended_workflow() else { return };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow: wf,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // When submitting a second run
        let Some(wf2) = suspended_workflow() else { return };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(2),
                workflow: wf2,
            }),
            Ok(())
        );
        // Then tick returns ActiveRunCapacityExceeded with capacity 1
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
    }

    #[test]
    fn shard_submit_creates_run_state_in_runs_map() {
        // Given a shard and a workflow
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else { return };
        let run = RunId::new(10);
        // When submitting a run
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // Then inspecting the run returns Found (proving it's in the runs map)
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 1,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let response = shard.take_inspect_response();
        match response {
            Some(InspectResponse::Found(snapshot)) => {
                assert_eq!(snapshot.run, run);
                assert_eq!(snapshot.correlation, 1);
            }
            other => {
                // Wrong: expected Found
                assert_eq!(other, None);
            }
        }
    }

    #[test]
    fn shard_submit_records_run_submitted_trace_event() {
        // Given a shard and a workflow
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else { return };
        let run = RunId::new(20);
        // When submitting a run
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // Then the trace ring contains a RunSubmitted event
        let events = shard.trace_ring_mut().drain();
        let found = events
            .iter()
            .any(|e| *e == TraceEvent::RunSubmitted { run });
        assert_eq!(found, true);
    }

    #[test]
    fn shard_submit_drives_run_immediately_for_finished_workflow() {
        // Given a shard and a finished workflow
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = finished_workflow() else { return };
        let run = RunId::new(30);
        // When submitting a run with a finishing workflow
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // Then the run is completed (not in runs map anymore) and counter shows completed
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
        // And inspect returns NotFound since the run finished
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 2,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.take_inspect_response(),
            Some(InspectResponse::NotFound { run, correlation: 2 })
        );
    }

    #[test]
    fn shard_resume_returns_error_for_unknown_run() {
        // Given a shard with no runs
        let config = small_config();
        let mut shard = Shard::new(config);
        // When resuming a non-existent run
        assert_eq!(
            shard.enqueue(ShardCommand::Resume {
                run: RunId::new(999),
            }),
            Ok(())
        );
        // Then tick returns RunNotFound
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_action_completed_returns_error_for_unknown_run() {
        // Given a shard with no runs
        let config = small_config();
        let mut shard = Shard::new(config);
        // When completing an action for a non-existent run
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted {
                run: RunId::new(888),
                step: StepIdx::new(0),
            }),
            Ok(())
        );
        // Then tick returns RunNotFound
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_action_completed_marks_step_succeeded() {
        // Given a shard with a suspended run (Do node at step 0)
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else { return };
        let run = RunId::new(55);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        let tick1 = shard.tick();
        // Then first tick succeeds (Do node suspends)
        assert_eq!(tick1, Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
        // When completing the action at step 0
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted {
                run,
                step: StepIdx::new(0),
            }),
            Ok(())
        );
        let tick2 = shard.tick();
        // Then second tick succeeds
        assert_eq!(tick2, Ok(true));
        // And the trace ring has an ActionCompleted event
        let events = shard.trace_ring_mut().drain();
        let found = events
            .iter()
            .any(|e| *e == TraceEvent::ActionCompleted {
                run,
                step: StepIdx::new(0),
            });
        assert_eq!(found, true);
    }

    #[test]
    fn shard_action_completed_records_trace_event() {
        // Given a shard with a suspended run
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else { return };
        let run = RunId::new(56);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // When completing the action
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted {
                run,
                step: StepIdx::new(0),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // Then the trace ring contains an ActionCompleted event
        let events = shard.trace_ring_mut().drain();
        let found = events
            .iter()
            .any(|e| *e == TraceEvent::ActionCompleted {
                run,
                step: StepIdx::new(0),
            });
        assert_eq!(found, true);
    }

    #[test]
    fn shard_timer_continues_suspended_run() {
        // Given a shard with a suspended run (Do node)
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else { return };
        let run = RunId::new(60);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // When timer fires for the run
        assert_eq!(
            shard.enqueue(ShardCommand::TimerFired { run }),
            Ok(())
        );
        // Then tick succeeds (run is re-driven)
        assert_eq!(shard.tick(), Ok(true));
    }

    #[test]
    fn shard_timer_returns_error_for_unknown_run() {
        // Given a shard with no runs
        let config = small_config();
        let mut shard = Shard::new(config);
        // When timer fires for a non-existent run
        assert_eq!(
            shard.enqueue(ShardCommand::TimerFired {
                run: RunId::new(777),
            }),
            Ok(())
        );
        // Then tick returns RunNotFound
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn shard_cancel_removes_run_from_runs_map() {
        // Given a shard with an active run
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else { return };
        let run = RunId::new(70);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // When cancelling the run
        assert_eq!(
            shard.enqueue(ShardCommand::Cancel { run }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // Then inspect returns NotFound (run removed from map)
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 5,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.take_inspect_response(),
            Some(InspectResponse::NotFound { run, correlation: 5 })
        );
    }

    #[test]
    fn shard_cancel_records_run_failed_trace_event() {
        // Given a shard with an active run
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else { return };
        let run = RunId::new(71);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // When cancelling the run
        assert_eq!(
            shard.enqueue(ShardCommand::Cancel { run }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // Then the trace ring contains a RunFailed event
        let events = shard.trace_ring_mut().drain();
        let found = events
            .iter()
            .any(|e| *e == TraceEvent::RunFailed { run });
        assert_eq!(found, true);
    }

    #[test]
    fn shard_cancel_increments_failed_counter() {
        // Given a shard with an active run
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else { return };
        let run = RunId::new(72);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // When cancelling the run
        assert_eq!(
            shard.enqueue(ShardCommand::Cancel { run }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // Then the failed counter is incremented
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
    }

    #[test]
    fn shard_inspect_captures_current_pc() {
        // Given a shard with an active suspended run at step 0
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else { return };
        let run = RunId::new(80);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // When inspecting the run
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 10,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // Then the snapshot pc matches the expected program counter
        match shard.take_inspect_response() {
            Some(InspectResponse::Found(snapshot)) => {
                assert_eq!(snapshot.pc, StepIdx::new(0));
                assert_eq!(snapshot.run, run);
                assert_eq!(snapshot.correlation, 10);
            }
            other => assert_eq!(other, None),
        }
    }

    #[test]
    fn shard_inspect_captures_executed_count() {
        // Given a shard with a finished workflow (executes 1 step)
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = finished_workflow() else { return };
        let run = RunId::new(81);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // Then the steps_executed counter reflects execution
        assert_eq!(shard.counters().snapshot().steps_executed, 1);
    }

    #[test]
    fn shard_tick_processes_commands_in_fifo_order() {
        // Given a shard with two submits enqueued
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(wf1) = finished_workflow() else { return };
        let Some(wf2) = suspended_workflow() else { return };
        // When submitting two runs
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(100),
                workflow: wf1,
            }),
            Ok(())
        );
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(101),
                workflow: wf2,
            }),
            Ok(())
        );
        // Then both ticks succeed in FIFO order
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.tick(), Ok(true));
        // And counters show both submitted
        assert_eq!(shard.counters().snapshot().runs_submitted, 2);
        // And the first run (finished workflow) is completed
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
    }

    #[test]
    fn shard_resume_continues_suspended_run() {
        // Given a shard with a suspended run (Do node at step 0)
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else { return };
        let run = RunId::new(90);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // When resuming the suspended run
        assert_eq!(
            shard.enqueue(ShardCommand::Resume { run }),
            Ok(())
        );
        // Then tick succeeds (run re-enters drive, suspends again on Do)
        assert_eq!(shard.tick(), Ok(true));
    }

    #[test]
    fn shard_take_inspect_response_returns_none_initially() {
        // Given a fresh shard
        let config = small_config();
        let mut shard = Shard::new(config);
        // When taking inspect response without any inspect command
        let response = shard.take_inspect_response();
        // Then response is None
        assert_eq!(response, None);
    }

    #[test]
    fn shard_take_inspect_response_clears_after_take() {
        // Given a shard with an inspect response available
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(workflow) = suspended_workflow() else { return };
        let run = RunId::new(95);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 1,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        // When taking the response
        let first = shard.take_inspect_response();
        assert_eq!(first.is_some(), true);
        // Then a second take returns None
        let second = shard.take_inspect_response();
        assert_eq!(second, None);
    }

    #[test]
    fn shard_is_shutting_down_defaults_to_false() {
        // Given a fresh shard
        let config = small_config();
        let shard = Shard::new(config);
        // Then is_shutting_down is false
        assert_eq!(shard.is_shutting_down(), false);
    }

    #[test]
    fn shard_config_default_values() {
        // Given a default ShardConfig
        let config = ShardConfig::default();
        // Then it has reasonable defaults
        assert_eq!(config.command_queue_capacity, 1024);
        assert_eq!(config.trace_capacity, 4096);
        assert_eq!(config.step_budget_per_tick, 1000);
        assert_eq!(config.max_active_runs, 1024);
    }

    #[test]
    fn shard_config_equality_same_values() {
        // Given two identical configs
        let a = ShardConfig::default();
        let b = ShardConfig::default();
        // Then they are equal
        assert_eq!(a, b);
    }

    #[test]
    fn shard_config_equality_differs() {
        // Given two different configs
        let a = ShardConfig::default();
        let b = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 1,
            step_budget_per_tick: 1,
            max_active_runs: 1,
        };
        // Then they are not equal
        assert_ne!(a, b);
    }

    #[test]
    fn shard_config_clone_preserves_values() {
        // Given a config
        let original = small_config();
        // When cloning
        let cloned = original.clone();
        // Then clone matches original
        assert_eq!(cloned, original);
    }

    #[test]
    fn shard_command_equality_submit() {
        // Given two identical Submit commands
        let Some(wf) = suspended_workflow() else { return };
        let a = ShardCommand::Submit { run: RunId::new(1), workflow: wf.clone() };
        let b = ShardCommand::Submit { run: RunId::new(1), workflow: wf };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_cancel() {
        // Given two identical Cancel commands
        let a = ShardCommand::Cancel { run: RunId::new(1) };
        let b = ShardCommand::Cancel { run: RunId::new(1) };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_differs_run_id() {
        // Given two Cancel commands with different run IDs
        let a = ShardCommand::Cancel { run: RunId::new(1) };
        let b = ShardCommand::Cancel { run: RunId::new(2) };
        assert_ne!(a, b);
    }

    #[test]
    fn shard_command_equality_shutdown() {
        // Given two Shutdown commands
        let a = ShardCommand::Shutdown;
        let b = ShardCommand::Shutdown;
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_inspect() {
        // Given two identical Inspect commands
        let a = ShardCommand::Inspect { run: RunId::new(1), correlation: 42 };
        let b = ShardCommand::Inspect { run: RunId::new(1), correlation: 42 };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_inspect_differs_correlation() {
        // Given two Inspect commands with different correlation
        let a = ShardCommand::Inspect { run: RunId::new(1), correlation: 1 };
        let b = ShardCommand::Inspect { run: RunId::new(1), correlation: 2 };
        assert_ne!(a, b);
    }

    #[test]
    fn shard_command_equality_action_completed() {
        // Given two identical ActionCompleted commands
        let a = ShardCommand::ActionCompleted { run: RunId::new(1), step: StepIdx::new(0) };
        let b = ShardCommand::ActionCompleted { run: RunId::new(1), step: StepIdx::new(0) };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_timer_fired() {
        // Given two identical TimerFired commands
        let a = ShardCommand::TimerFired { run: RunId::new(1) };
        let b = ShardCommand::TimerFired { run: RunId::new(1) };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_command_equality_resume() {
        // Given two identical Resume commands
        let a = ShardCommand::Resume { run: RunId::new(1) };
        let b = ShardCommand::Resume { run: RunId::new(1) };
        assert_eq!(a, b);
    }

    #[test]
    fn shard_cancel_nonexistent_does_not_increment_failed() {
        // Given a shard
        let config = small_config();
        let mut shard = Shard::new(config);
        // When cancelling a non-existent run
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run: RunId::new(999) }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        // Then the failed counter is NOT incremented (run didn't exist)
        assert_eq!(shard.counters().snapshot().runs_failed, 0);
    }

    #[test]
    fn shard_finished_workflow_sets_completed_counter() {
        // Given a shard with a finished workflow
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(wf) = finished_workflow() else { return };
        let run = RunId::new(50);
        assert_eq!(shard.enqueue(ShardCommand::Submit { run, workflow: wf }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        // Then completed counter is 1
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    }

    #[test]
    fn shard_finished_workflow_produces_run_finished_trace() {
        // Given a shard with a finished workflow
        let config = small_config();
        let mut shard = Shard::new(config);
        let Some(wf) = finished_workflow() else { return };
        let run = RunId::new(51);
        assert_eq!(shard.enqueue(ShardCommand::Submit { run, workflow: wf }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        // Then the trace contains RunFinished
        let events = shard.trace_ring_mut().drain();
        let found = events.iter().any(|e| *e == TraceEvent::RunFinished { run });
        assert_eq!(found, true);
    }

    #[test]
    fn shard_inspect_response_not_found_for_unknown_run() {
        // Given a shard with no runs
        let config = small_config();
        let mut shard = Shard::new(config);
        // When inspecting a non-existent run
        assert_eq!(shard.enqueue(ShardCommand::Inspect { run: RunId::new(999), correlation: 1 }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        // Then response is NotFound
        assert_eq!(
            shard.take_inspect_response(),
            Some(InspectResponse::NotFound { run: RunId::new(999), correlation: 1 })
        );
    }

    #[test]
    fn inspect_response_found_equality() {
        // Given two identical Found responses
        let a = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(1),
            correlation: 42,
            pc: StepIdx::new(0),
            executed: 5,
        });
        let b = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(1),
            correlation: 42,
            pc: StepIdx::new(0),
            executed: 5,
        });
        assert_eq!(a, b);
    }

    #[test]
    fn inspect_response_found_differs_executed() {
        // Given two Found responses with different executed counts
        let a = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(1),
            correlation: 1,
            pc: StepIdx::new(0),
            executed: 5,
        });
        let b = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(1),
            correlation: 1,
            pc: StepIdx::new(0),
            executed: 10,
        });
        assert_ne!(a, b);
    }

    #[test]
    fn inspect_response_not_found_equality() {
        // Given two identical NotFound responses
        let a = InspectResponse::NotFound { run: RunId::new(1), correlation: 42 };
        let b = InspectResponse::NotFound { run: RunId::new(1), correlation: 42 };
        assert_eq!(a, b);
    }

    #[test]
    fn run_state_equality() {
        // Given a suspended workflow and run frame
        let Some(wf) = suspended_workflow() else { return };
        let frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 1) {
            Ok(f) => f,
            Err(_) => return,
        };
        let state = RunState {
            frame,
            workflow: wf.clone(),
            store: ValueStore::new(),
        };
        let frame2 = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 1) {
            Ok(f) => f,
            Err(_) => return,
        };
        let state2 = RunState {
            frame: frame2,
            workflow: wf,
            store: ValueStore::new(),
        };
        assert_eq!(state, state2);
    }
}
