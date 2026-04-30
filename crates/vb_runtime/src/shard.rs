//! Single-threaded shard owning mutable run state directly.

use crossbeam_queue::ArrayQueue;
use std::collections::HashMap;
use vb_core::engine::{EngineSignal, StepBudget, new_run_frame, run_until_blocked};
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::counters::ShardCounters;
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
        self.inspect_response = match self.runs.get(&run) {
            Some(state) => Some(InspectResponse::Found(InspectSnapshot {
                run,
                correlation,
                pc: state.frame.pc(),
                executed: state.frame.executed(),
            })),
            None => Some(InspectResponse::NotFound { run, correlation }),
        };
    }

    fn drive_run(&mut self, run: RunId) -> RuntimeResult<()> {
        let state = self.runs.remove(&run);
        let Some(mut state) = state else {
            return Err(RuntimeError::RunNotFound);
        };

        let budget = StepBudget::new(self.step_budget_per_tick);
        let result = run_until_blocked(&state.workflow, &mut state.frame, budget, &mut state.store);

        match result {
            Ok(EngineSignal::Continue) => {
                self.counters.add_steps(state.frame.executed());
                self.runs.insert(run, state);
            }
            Ok(EngineSignal::Finished(_)) => {
                self.counters.inc_completed();
                self.counters.add_steps(state.frame.executed());
                self.trace_ring.push(TraceEvent::RunFinished { run });
            }
            Ok(EngineSignal::StepBudgetExhausted) => {
                self.counters.add_steps(state.frame.executed());
                self.runs.insert(run, state);
            }
            Ok(EngineSignal::AwaitingAction) => {
                self.counters.add_steps(state.frame.executed());
                let step = state.frame.pc();
                self.trace_ring
                    .push(TraceEvent::ActionScheduled { run, step });
                self.runs.insert(run, state);
            }
            Ok(EngineSignal::AwaitingWait) | Ok(EngineSignal::AwaitingAsk) => {
                self.counters.add_steps(state.frame.executed());
                self.runs.insert(run, state);
            }
            Err(_) => {
                self.counters.inc_failed();
                self.trace_ring.push(TraceEvent::RunFailed { run });
            }
        }

        Ok(())
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
    use vb_core::ids::{ActionId, SlotIdx, WorkflowDigest};
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
}
