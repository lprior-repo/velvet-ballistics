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

/// Single-threaded shard owning all mutable run state.
pub struct Shard {
    command_queue: ArrayQueue<ShardCommand>,
    runs: HashMap<RunId, RunState>,
    trace_ring: TraceRing,
    counters: ShardCounters,
    step_budget_per_tick: u64,
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
            ShardCommand::ActionCompleted { run, step } => self.handle_action_completion(run, step)?,
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

    /// Returns true if the shard is shutting down.
    #[must_use]
    pub const fn is_shutting_down(&self) -> bool {
        self.shutting_down
    }

    fn handle_submit(&mut self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        let frame = new_run_frame(run, &workflow)
            .map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring.push(TraceEvent::RunSubmitted { run });
        self.counters.inc_submitted();
        let state = RunState { frame, workflow, store: ValueStore::new() };
        self.runs.insert(run, state);
        self.drive_run(run)?;
        Ok(())
    }

    fn handle_resume(&mut self, run: RunId) -> RuntimeResult<()> {
        self.drive_run(run)
    }

    fn handle_action_completion(&mut self, run: RunId, step: StepIdx) -> RuntimeResult<()> {
        let state = self
            .runs
            .get_mut(&run)
            .ok_or(RuntimeError::RunNotFound)?;
        state.frame.mark_succeeded(step).map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring.push(TraceEvent::ActionCompleted { run, step });
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
        if let Some(state) = self.runs.get(&run) {
            self.trace_ring.push(TraceEvent::StepStarted {
                run,
                step: state.frame.pc(),
            });
            let _snapshot = InspectSnapshot {
                run,
                correlation,
                pc: state.frame.pc(),
                executed: state.frame.executed(),
            };
        }
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
                self.trace_ring.push(TraceEvent::ActionScheduled { run, step });
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
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            command_queue_capacity: 1024,
            trace_capacity: 4096,
            step_budget_per_tick: 1000,
        }
    }
}
