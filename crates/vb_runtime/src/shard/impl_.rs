//! Shard construction, queue operations, and core tick processing.

use crossbeam_queue::ArrayQueue;
use indexmap::IndexMap;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx};
use vb_core::workflow::CompiledWorkflow;

use crate::counters::ShardCounters;
use crate::engine::{EvidenceCollector, EvidenceEvent};
use crate::frame_pool::FramePool;
use crate::journal::{NoopRuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal};
use crate::trace::{TraceEvent, TraceRing};
use crate::{RuntimeError, RuntimeResult};

use crate::shard::types::{
    InspectResponse, MAX_COMMAND_QUEUE_CAPACITY, Shard, ShardCommand, ShardConfig,
};

impl Shard {
    /// Creates a new shard with the given configuration.
    pub fn new(config: ShardConfig) -> Self {
        Self::new_with_journal(config, NoopRuntimeJournal::shared())
    }

    /// Creates a new shard with the given configuration, journal sink, and artifact store.
    pub fn new_with_journal_and_artifact_store(
        config: ShardConfig,
        journal: SharedRuntimeJournal,
        artifact_store: crate::admission::SharedArtifactStore,
    ) -> Self {
        Self {
            command_queue: ArrayQueue::new(config.command_queue_capacity),
            runs: IndexMap::new(),
            pending_timers: IndexMap::new(),
            frame_pools: IndexMap::new(),
            trace_ring: TraceRing::new(config.trace_capacity),
            counters: ShardCounters::new(),
            step_budget_per_tick: config.step_budget_per_tick,
            max_active_runs: config.max_active_runs,
            policy: config.policy,
            artifact_store,
            inspect_response: None,
            shutting_down: false,
            journal,
        }
    }

    /// Creates a new shard with the given configuration and journal sink.
    pub fn new_with_journal(config: ShardConfig, journal: SharedRuntimeJournal) -> Self {
        Self::new_with_journal_and_artifact_store(
            config,
            journal,
            crate::admission::AlwaysPresentArtifactStore::shared(),
        )
    }

    /// Enqueues a command. Returns `QueueFull` on overflow.
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        self.command_queue
            .push(cmd)
            .map_err(|_| RuntimeError::QueueFull)
    }

    /// Returns the number of commands currently in the queue.
    #[must_use]
    pub fn command_queue_len(&self) -> usize {
        self.command_queue.len()
    }

    /// Returns the remaining free slots in the command queue.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.command_queue
            .capacity()
            .saturating_sub(self.command_queue.len())
    }

    /// Returns true if the command queue is full.
    #[must_use]
    pub fn is_queue_full(&self) -> bool {
        self.command_queue.is_full()
    }

    /// Returns the command queue capacity.
    #[must_use]
    pub fn command_queue_capacity(&self) -> usize {
        self.command_queue.capacity()
    }

    /// Returns the number of active runs on this shard.
    #[must_use]
    pub fn active_run_count(&self) -> usize {
        self.runs.len()
    }

    /// Returns the number of pending timers on this shard.
    #[must_use]
    pub fn pending_timer_count(&self) -> usize {
        self.pending_timers.len()
    }

    /// Returns frame pool metrics across all pools: (free, total_capacity).
    #[must_use]
    pub fn frame_pool_metrics(&self) -> (usize, usize) {
        let mut free = 0usize;
        let mut total = 0usize;
        for pool in self.frame_pools.values() {
            free = free.saturating_add(pool.available());
            total = total.saturating_add(pool.capacity());
        }
        (free, total)
    }

    /// Processes one command from the queue. Returns false if the shard should shut down.
    pub fn tick(&mut self) -> RuntimeResult<bool> {
        if self.shutting_down {
            return Ok(false);
        }

        let Some(cmd) = self.command_queue.pop() else {
            return Ok(true);
        };

        match cmd {
            ShardCommand::Submit {
                run,
                workflow,
                caps,
            } => self.handle_submit(run, workflow, caps)?,
            ShardCommand::SubmitWithInputs {
                run,
                workflow,
                inputs,
                caps,
            } => self.handle_submit_with_inputs(run, workflow, &inputs, caps)?,
            ShardCommand::Resume { run } => self.handle_resume(run)?,
            ShardCommand::ActionCompleted { ticket, output } => {
                self.handle_action_completion(ticket, output)?;
            }
            ShardCommand::ActionCompletedLegacy { run, step } => {
                self.handle_legacy_action_completion(run, step)?;
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
            Some(state) => InspectResponse::Found(crate::shard::helpers::snapshot_from_state(
                run,
                correlation,
                state,
            )),
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

    /// Drains evidence events from the collector and emits them to the
    /// journal and trace ring. This satisfies the Phase 40/44 evidence
    /// chain requirement: StepStarted before SlotWritten for every step,
    /// followed by StepSucceeded.
    pub(crate) fn flush_evidence(
        &mut self,
        run: RunId,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeResult<()> {
        let events = evidence.drain();
        for ev in events {
            match ev {
                EvidenceEvent::StepStarted { step } => {
                    self.trace_ring.push(TraceEvent::StepStarted { run, step });
                    self.journal
                        .append(RuntimeJournalEvent::StepStarted { run, step })?;
                }
                EvidenceEvent::StepSucceeded { step, output } => {
                    if let Some(slot) = output {
                        self.trace_ring.push(TraceEvent::SlotWritten { run, slot });
                        // Get the frame to read the slot value
                        if let Some(state) = self.runs.get(&run) {
                            if let Ok(value) = state.frame.read_slot(slot) {
                                let encoded = postcard::to_allocvec(value)
                                    .map_err(|_| RuntimeError::EncodeFailed)?;
                                self.journal.append(RuntimeJournalEvent::SlotWritten {
                                    run,
                                    slot,
                                    value: encoded,
                                })?;
                            } else {
                                self.journal.append(RuntimeJournalEvent::SlotWritten {
                                    run,
                                    slot,
                                    value: vec![],
                                })?;
                            }
                        } else {
                            self.journal.append(RuntimeJournalEvent::SlotWritten {
                                run,
                                slot,
                                value: vec![],
                            })?;
                        }
                    }
                    self.journal.append(RuntimeJournalEvent::StepSucceeded {
                        run,
                        step,
                        output: output.unwrap_or(SlotIdx::ZERO),
                    })?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn take_frame_for(
        &mut self,
        run: RunId,
        workflow: &CompiledWorkflow,
    ) -> RuntimeResult<RunFrame> {
        let step_count = workflow.node_count();
        let slot_count = workflow.slot_count();
        let key = (step_count, slot_count);
        if !self.frame_pools.contains_key(&key) {
            let pool = FramePool::new(step_count, slot_count, self.max_active_runs)
                .map_err(|_| RuntimeError::FramePoolUnavailable)?;
            self.frame_pools.insert(key, pool);
        }
        let pool = self
            .frame_pools
            .get_mut(&key)
            .ok_or(RuntimeError::FramePoolUnavailable)?;
        pool.take(run, workflow.entry())
            .map_err(|_| RuntimeError::FramePoolUnavailable)
    }

    pub(crate) fn release_frame(&mut self, frame: RunFrame) {
        let key = (frame.step_count(), frame.slot_count());
        if let Some(pool) = self.frame_pools.get_mut(&key) {
            pool.release(frame);
        }
    }

    /// Drains the command queue by processing commands until shutdown or capacity limit.
    pub fn drain_for_shutdown(&mut self) -> RuntimeResult<()> {
        let limit = self.command_queue.capacity();
        let mut processed = 0usize;
        while processed < limit {
            if !self.tick()? {
                return Ok(());
            }
            processed = processed.saturating_add(1);
        }
        Err(RuntimeError::ShutdownInProgress)
    }
}

/// ShardConfig validation and construction.
impl ShardConfig {
    /// Creates a new ShardConfig, validating capacity limits.
    pub fn new(
        command_queue_capacity: usize,
        trace_capacity: usize,
        step_budget_per_tick: u64,
        max_active_runs: usize,
        policy: vb_core::policy::RuntimePolicy,
    ) -> RuntimeResult<Self> {
        if command_queue_capacity == 0 || command_queue_capacity > MAX_COMMAND_QUEUE_CAPACITY {
            return Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: command_queue_capacity,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            });
        }
        if max_active_runs == 0 {
            return Err(RuntimeError::ActiveRunCapacityZero);
        }
        Ok(Self {
            command_queue_capacity,
            trace_capacity,
            step_budget_per_tick,
            max_active_runs,
            policy,
        })
    }
}
