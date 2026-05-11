#![forbid(unsafe_code)]
//! Shard construction, queue operations, and core tick processing.

use crossbeam_queue::ArrayQueue;
use indexmap::IndexMap;
use vb_core::action::ActionContract;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::workflow::CompiledWorkflow;

use crate::counters::ShardCounters;
use crate::engine::{EvidenceCollector, EvidenceEvent};
use crate::frame_pool::FramePool;
use crate::journal::{NoopRuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal};
use crate::trace::{TraceEvent, TraceRing};
use crate::{RuntimeError, RuntimeResult};
use vb_storage::recovery::ActionReplayTracker;

use crate::shard::types::{
    InspectResponse, MAX_COMMAND_QUEUE_CAPACITY, Shard, ShardCommand, ShardConfig, ShardHealth,
    ShardStatus,
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
        artifact_store: crate::admission::SharedAcceptedArtifactStore,
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
            action_contracts: Vec::new(),
            replay_tracker: ActionReplayTracker::new(),
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

    /// Sets the action contracts for idempotency policy lookups.
    /// Should be called during shard initialization before processing runs.
    pub fn set_action_contracts(&mut self, contracts: Vec<ActionContract>) {
        self.action_contracts = contracts;
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

    /// Returns a read-only status snapshot without draining queues or mutating shard state.
    #[must_use]
    pub fn status(&self) -> ShardStatus {
        let shutting_down = self.shutting_down;
        ShardStatus {
            health: if shutting_down {
                ShardHealth::ShuttingDown
            } else {
                ShardHealth::Running
            },
            running: !shutting_down,
            shutting_down,
            command_queue_depth: self.command_queue.len(),
            command_queue_capacity: self.command_queue.capacity(),
            active_runs: self.runs.len(),
            max_active_runs: self.max_active_runs,
            trace_capacity: self.trace_ring.capacity(),
            trace_dropped: self.trace_ring.dropped(),
            step_budget_per_tick: self.step_budget_per_tick,
            runtime_policy: self.policy,
        }
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
        evidence
            .drain()
            .into_iter()
            .try_for_each(|event| self.flush_evidence_event(run, event))
    }

    fn flush_evidence_event(&mut self, run: RunId, event: EvidenceEvent) -> RuntimeResult<()> {
        match event {
            EvidenceEvent::StepStarted { step } => self.flush_step_started(run, step),
            EvidenceEvent::StepSucceeded { step, output } => {
                self.flush_step_succeeded(run, step, output)
            }
            EvidenceEvent::SlotWritten {
                slot,
                value,
                taint,
                extra,
            } => self.flush_slot_written(run, slot, value, taint, extra),
        }
    }

    fn flush_step_started(&mut self, run: RunId, step: StepIdx) -> RuntimeResult<()> {
        self.trace_ring.push(TraceEvent::StepStarted { run, step });
        self.journal
            .append(RuntimeJournalEvent::StepStarted { run, step })
    }

    fn flush_step_succeeded(
        &mut self,
        run: RunId,
        step: StepIdx,
        output: Option<SlotIdx>,
    ) -> RuntimeResult<()> {
        self.journal.append(RuntimeJournalEvent::StepSucceeded {
            run,
            step,
            output: match output {
                Some(slot) => slot,
                None => SlotIdx::ZERO,
            },
        })
    }

    fn flush_slot_written(
        &mut self,
        run: RunId,
        slot: SlotIdx,
        value: vb_core::value::SlotValue,
        taint: vb_core::Taint,
        extra: Option<crate::primitives::collect::CollectPaginationState>,
    ) -> RuntimeResult<()> {
        let encoded = postcard::to_allocvec(&value).map_err(|_| RuntimeError::EncodeFailed)?;
        let encoded_extra = extra
            .map(|state| postcard::to_allocvec(&state))
            .transpose()
            .map_err(|_| RuntimeError::EncodeFailed)?;
        self.trace_ring.push(TraceEvent::SlotWritten {
            run,
            slot,
            value: encoded.clone(),
        });
        self.journal.append(RuntimeJournalEvent::SlotWritten {
            run,
            slot,
            value: encoded,
            taint,
            extra: encoded_extra,
        })
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
                self.pending_timers.clear();
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

#[cfg(test)]
mod tests {
    use vb_core::capability::CapabilitySet;
    use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::ConstValue;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    use crate::RuntimeError;

    use super::{MAX_COMMAND_QUEUE_CAPACITY, Shard, ShardCommand, ShardConfig, ShardHealth};

    fn finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_const = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
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
            constants: Box::from([ConstValue::Bool(true)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn small_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        }
    }

    // =======================================================================
    // ShardConfig::new validation
    // =======================================================================

    #[test]
    fn config_new_accepts_min_valid_capacity() {
        let result = ShardConfig::new(1, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
        let expected = ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 1,
            step_budget_per_tick: 1,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        assert_eq!(result, Ok(expected));
    }

    #[test]
    fn config_new_rejects_zero_capacity() {
        let result = ShardConfig::new(0, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            result,
            Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: 0,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            })
        );
    }

    #[test]
    fn config_new_rejects_capacity_exceeding_max() {
        let too_large = MAX_COMMAND_QUEUE_CAPACITY.saturating_add(1);
        let result = ShardConfig::new(too_large, 1, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            result,
            Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: too_large,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            })
        );
    }

    #[test]
    fn config_new_rejects_zero_max_active_runs() {
        let result = ShardConfig::new(1, 1, 1, 0, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(result, Err(RuntimeError::ActiveRunCapacityZero));
    }

    #[test]
    fn config_new_accepts_max_command_queue_capacity() {
        let result = ShardConfig::new(
            MAX_COMMAND_QUEUE_CAPACITY,
            1,
            1,
            1,
            vb_core::policy::RuntimePolicy::Relaxed,
        );
        let expected = ShardConfig {
            command_queue_capacity: MAX_COMMAND_QUEUE_CAPACITY,
            trace_capacity: 1,
            step_budget_per_tick: 1,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        assert_eq!(result, Ok(expected));
    }

    #[test]
    fn config_new_preserves_all_fields() {
        let config = ShardConfig::new(64, 128, 256, 32, vb_core::policy::RuntimePolicy::Relaxed);
        assert_eq!(
            config,
            Ok(ShardConfig {
                command_queue_capacity: 64,
                trace_capacity: 128,
                step_budget_per_tick: 256,
                max_active_runs: 32,
                policy: vb_core::policy::RuntimePolicy::Relaxed,
            })
        );
    }

    // =======================================================================
    // Shard construction
    // =======================================================================

    #[test]
    fn shard_new_creates_empty_shard() {
        let shard = Shard::new(small_config());
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.pending_timer_count(), 0);
        assert_eq!(shard.command_queue_len(), 0);
        assert_eq!(shard.is_shutting_down(), false);
    }

    // =======================================================================
    // Queue operations
    // =======================================================================

    #[test]
    fn enqueue_and_capacity_tracking() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.command_queue_capacity(), 4);
        assert_eq!(shard.remaining_capacity(), 4);
        assert_eq!(shard.is_queue_full(), false);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.command_queue_len(), 1);
        assert_eq!(shard.remaining_capacity(), 3);
    }

    #[test]
    fn queue_full_at_capacity_boundary() {
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let shard = Shard::new(config);
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.is_queue_full(), true);
        assert_eq!(shard.remaining_capacity(), 0);
        assert_eq!(
            shard.enqueue(ShardCommand::Shutdown),
            Err(RuntimeError::QueueFull)
        );
    }

    // =======================================================================
    // Tick processing
    // =======================================================================

    #[test]
    fn tick_on_empty_queue_returns_true() {
        let mut shard = Shard::new(small_config());
        assert_eq!(shard.tick(), Ok(true));
    }

    #[test]
    fn tick_processes_shutdown_returns_false() {
        let mut shard = Shard::new(small_config());
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.is_shutting_down(), true);
    }

    #[test]
    fn tick_after_shutdown_always_returns_false() {
        let mut shard = Shard::new(small_config());
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));
        assert_eq!(shard.tick(), Ok(false));
    }

    // =======================================================================
    // drain_for_shutdown
    // =======================================================================

    #[test]
    fn drain_for_shutdown_processes_pending_commands() {
        let config = ShardConfig {
            command_queue_capacity: 4,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        let Some(wf) = finished_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.drain_for_shutdown(), Ok(()));
        assert_eq!(shard.is_shutting_down(), true);
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
    }

    #[test]
    fn drain_for_shutdown_on_empty_queue_hits_capacity_limit() {
        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        assert_eq!(
            shard.drain_for_shutdown(),
            Err(RuntimeError::ShutdownInProgress)
        );
    }

    // =======================================================================
    // snapshot_run (direct, non-queued)
    // =======================================================================

    #[test]
    fn snapshot_run_returns_not_found_for_missing_run() {
        let shard = Shard::new(small_config());
        let response = shard.snapshot_run(RunId::new(999), 42);
        match response {
            super::InspectResponse::NotFound { run, correlation } => {
                assert_eq!(run, RunId::new(999));
                assert_eq!(correlation, 42);
            }
            other => {
                assert_eq!(
                    other,
                    super::InspectResponse::NotFound {
                        run: RunId::new(999),
                        correlation: 42,
                    }
                );
            }
        }
    }

    fn submit_finished_run(shard: &mut Shard, run: RunId) {
        let Some(wf) = finished_workflow() else {
            assert_eq!(None::<()>, Some(()));
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    // =======================================================================
    // Frame pool metrics
    // =======================================================================

    #[test]
    fn frame_pool_metrics_zero_initially() {
        let shard = Shard::new(small_config());
        let (free, total) = shard.frame_pool_metrics();
        assert_eq!(free, 0);
        assert_eq!(total, 0);
    }

    // =======================================================================
    // Boundary conditions
    // =======================================================================

    #[test]
    fn shard_with_run_id_zero() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(0),
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
    }

    #[test]
    fn shard_with_max_run_id() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(u64::MAX),
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
    }

    #[test]
    fn shard_handles_multiple_sequential_finished_runs() {
        let mut shard = Shard::new(small_config());
        submit_finished_run(&mut shard, RunId::new(0));
        submit_finished_run(&mut shard, RunId::new(1));
        submit_finished_run(&mut shard, RunId::new(2));
        submit_finished_run(&mut shard, RunId::new(3));
        assert_eq!(shard.counters().snapshot().runs_completed, 4);
        assert_eq!(shard.counters().snapshot().runs_submitted, 4);
    }

    #[test]
    fn take_inspect_response_returns_none_when_none_pending() {
        let mut shard = Shard::new(small_config());
        assert_eq!(shard.take_inspect_response(), None);
    }

    #[test]
    fn status_reports_shard_health_and_capacity_without_mutation() {
        let shard = Shard::new(small_config());
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        let before_len = shard.command_queue_len();

        let status = shard.status();

        assert_eq!(status.health, ShardHealth::Running);
        assert_eq!(status.running, true);
        assert_eq!(status.shutting_down, false);
        assert_eq!(status.command_queue_depth, 1);
        assert_eq!(status.command_queue_capacity, 16);
        assert_eq!(status.active_runs, 0);
        assert_eq!(status.max_active_runs, 4);
        assert_eq!(status.trace_capacity, 16);
        assert_eq!(status.trace_dropped, 0);
        assert_eq!(status.step_budget_per_tick, 4);
        assert_eq!(
            status.runtime_policy,
            vb_core::policy::RuntimePolicy::Relaxed
        );
        assert_eq!(shard.command_queue_len(), before_len);
    }

    #[test]
    fn status_reports_shutting_down_after_shutdown_tick() {
        let mut shard = Shard::new(small_config());
        assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
        assert_eq!(shard.tick(), Ok(false));

        let status = shard.status();

        assert_eq!(status.health, ShardHealth::ShuttingDown);
        assert_eq!(status.running, false);
        assert_eq!(status.shutting_down, true);
    }
}
