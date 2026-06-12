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

mod runtime_metrics;
use runtime_metrics::collect_metrics;
mod runtime_control;
pub use runtime_control::ActiveRunSummary;
mod runtime_admission;
mod runtime_ask;

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

    /// Creates a runtime with an explicit artifact store.
    ///
    /// Test-support constructor used by admission tests to wire a
    /// `AlwaysPresentArtifactStore` into strict-mode admission so the
    /// step-budget gate can be evaluated in isolation.
    #[cfg(any(test, feature = "test-util"))]
    #[must_use]
    pub fn new_with_artifact_store(
        shard_count: NonZeroUsize,
        config: ShardConfig,
        artifact_store: crate::admission::SharedAcceptedArtifactStore,
    ) -> Self {
        let journal = crate::journal::NoopRuntimeJournal::shared();
        let count = shard_count.get();
        let mut shards = Vec::with_capacity(count);
        let mut index = 0usize;
        while index < count {
            shards.push(Shard::new_with_journal_and_artifact_store(
                config,
                journal.clone(),
                crate::admission::SharedAcceptedArtifactStore::clone(&artifact_store),
            ));
            index = index.saturating_add(1);
        }
        Self {
            shards,
            shard_count: count,
            journal,
        }
    }

    /// Submits a run using a compiled workflow.
    ///
    /// Admission is atomic with the enqueue: the per-shard `admission_lock`
    /// is held for the duration of the preflight and the enqueue so two
    /// concurrent submits cannot squeeze in between the budget reservation
    /// and the queue commit. Fails closed if the workflow's step count
    /// exceeds `vb_core::limits::MAX_STEPS_PER_WORKFLOW`.
    pub fn submit_direct(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        let _admission_guard = shard.lock_admission()?;
        Self::preflight_direct_admission(shard, run, &workflow, CapabilitySet::empty())?;
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: CapabilitySet::empty(),
        })
    }

    pub fn submit_compiled(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        self.submit_direct(run, workflow)
    }

    /// Submits a run with pre-mapped runtime input slots.
    ///
    /// Admission is atomic with the enqueue via the per-shard
    /// `admission_lock`. The preflight now enforces BOTH the artifact gate
    /// and the per-workflow step-count policy.
    pub fn submit_compiled_with_inputs(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        let _admission_guard = shard.lock_admission()?;
        Self::preflight_direct_admission(shard, run, &workflow, CapabilitySet::empty())?;
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs,
            caps: CapabilitySet::empty(),
        })
    }

    /// Submits a run with inputs, capability grants, and validated action
    /// contracts.
    ///
    /// Admission is atomic with the enqueue via the per-shard
    /// `admission_lock`. The preflight now enforces BOTH the artifact gate
    /// and the per-workflow step-count policy. Fails closed on either gate.
    pub fn submit_direct_with_inputs_grants_and_contracts(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
        caps: CapabilitySet,
        action_contracts: Box<[vb_core::action::ActionContract]>,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        let _admission_guard = shard.lock_admission()?;
        Self::preflight_direct_admission(shard, run, &workflow, caps.clone())?;
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
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
            .enqueue(ShardCommand::RuntimeActionFailed { ticket, failure })
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

    /// Recovers all incomplete runs from the durable journal and rehydrates
    /// pending timers. Returns the list of rehydrated run IDs for observability.
    ///
    /// This is a hard, atomic operation: any error in hydration is propagated
    /// without partial state. On success, the runtime is ready to resume work
    /// across the new process boundary.
    ///
    /// Requires the `test-util` feature on `vb_core` (gated via `vb_runtime/test-util`).
    #[cfg(feature = "test-util")]
    pub fn recover(
        &mut self,
        journal: &crate::journal::SharedRuntimeJournal,
    ) -> RuntimeResult<Vec<RunId>> {
        let hydrations = vb_storage::recovery::recover_all_incomplete_runs(
            journal
                .storage_journal()
                .ok_or(RuntimeError::InvalidRecoveryHydration)?
                .as_ref(),
        )
        .map_err(|_| RuntimeError::InvalidRecoveryHydration)?;

        let mut recovered = Vec::with_capacity(hydrations.len());
        for hydration in hydrations {
            if let Some(run) = self.recover_one_run(journal, hydration)? {
                recovered.push(run);
            }
        }
        Ok(recovered)
    }

    /// Recovers a single run from a hydration seed.
    /// Returns the run ID if recovered, `None` if skipped.
    #[cfg(feature = "test-util")]
    fn recover_one_run(
        &mut self,
        journal: &crate::journal::SharedRuntimeJournal,
        hydration: vb_storage::recovery::RecoveryHydration,
    ) -> RuntimeResult<Option<vb_core::ids::RunId>> {
        let seed = match hydration {
            vb_storage::recovery::RecoveryHydration::FrameSeed(s) => s,
            _ => return Ok(None),
        };
        let run = seed.summary.run;
        let slot_count = seed.slot_count;
        let pc = seed.pc;
        let frame = Self::hydrate_frame(seed)?;
        let pending_timer = Self::recover_timer_from_journal(journal, run, pc)?;
        Self::rehydrate_run_state(self, run, frame, slot_count, pending_timer)?;
        Ok(Some(run))
    }

    /// Hydrates a run frame from a recovery seed.
    #[cfg(feature = "test-util")]
    fn hydrate_frame(
        seed: vb_storage::recovery::RecoveryFrameSeed,
    ) -> RuntimeResult<vb_core::frame::RunFrame> {
        use crate::recovery::{DurableFrameRecoveryBoundary, RuntimeRecoveryBoundary};
        let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
        boundary.hydrate_run_frame()
    }

    /// Scans a run's journal for the last WaitScheduled or AskScheduled event.
    #[cfg(feature = "test-util")]
    fn find_timer_event(
        events: &[vb_storage::JournalEvent],
        pc: vb_core::StepIdx,
    ) -> Option<(&vb_storage::JournalEvent, vb_core::StepIdx)> {
        events
            .iter()
            .rev()
            .find(|ev| Self::event_matches_step(ev, pc))
            .map(|ev| (ev, pc))
    }

    #[cfg(feature = "test-util")]
    fn event_matches_step(ev: &vb_storage::JournalEvent, pc: vb_core::StepIdx) -> bool {
        match ev {
            vb_storage::JournalEvent::WaitScheduledEvent { step: s, .. }
            | vb_storage::JournalEvent::AskScheduledEvent { step: s, .. } => pc == *s,
            _ => false,
        }
    }

    /// Extracts pending timer info from journal events for a suspended run.
    #[cfg(feature = "test-util")]
    fn recover_timer_from_journal(
        journal: &crate::journal::SharedRuntimeJournal,
        run: vb_core::ids::RunId,
        pc: vb_core::StepIdx,
    ) -> RuntimeResult<Option<crate::shard::PendingTimer>> {
        let events = journal
            .storage_journal()
            .ok_or(RuntimeError::InvalidRecoveryHydration)?
            .events_for_run(run)
            .map_err(|_| RuntimeError::InvalidRecoveryHydration)?;
        Ok(Self::build_timer_from_event(Self::find_timer_event(
            &events, pc,
        )))
    }

    #[cfg(feature = "test-util")]
    fn build_timer_from_event(
        event: Option<(&vb_storage::JournalEvent, vb_core::StepIdx)>,
    ) -> Option<crate::shard::PendingTimer> {
        use crate::shard::timer::PendingTimerKind;
        event.and_then(|(ev, pc)| match ev {
            vb_storage::JournalEvent::WaitScheduledEvent {
                step: s,
                deadline_ms,
                ..
            } if pc == *s => Some(Self::make_timer(*s, PendingTimerKind::Wait, *deadline_ms)),
            vb_storage::JournalEvent::AskScheduledEvent {
                step: s,
                deadline_ms,
                ..
            } if pc == *s => Some(Self::make_timer(*s, PendingTimerKind::Ask, *deadline_ms)),
            _ => None,
        })
    }

    #[cfg(feature = "test-util")]
    fn make_timer(
        step: vb_core::StepIdx,
        kind: crate::shard::timer::PendingTimerKind,
        deadline_ms: u64,
    ) -> crate::shard::PendingTimer {
        crate::shard::PendingTimer {
            step,
            kind,
            generation: 0, // Updated by insert_timer
            deadline: std::time::Instant::now()
                .checked_add(std::time::Duration::from_millis(deadline_ms))
                .unwrap_or_else(std::time::Instant::now),
        }
    }

    /// Rehydrates a single run into its shard.
    #[cfg(feature = "test-util")]
    fn rehydrate_run_state(
        &mut self,
        run: vb_core::ids::RunId,
        frame: vb_core::frame::RunFrame,
        slot_count: u16,
        pending_timer: Option<crate::shard::PendingTimer>,
    ) -> RuntimeResult<()> {
        let shard_idx = self.shard_index(run);
        {
            let shard = self
                .shards
                .get_mut(shard_idx)
                .ok_or(RuntimeError::RunNotFound)?;
            Self::insert_into_shard(shard, run, frame, slot_count);
        }
        if let Some(timer) = pending_timer {
            Self::insert_timer(self, run, shard_idx, timer)?;
        }
        Ok(())
    }

    #[cfg(feature = "test-util")]
    fn insert_into_shard(
        shard: &mut Shard,
        run: vb_core::ids::RunId,
        frame: vb_core::frame::RunFrame,
        slot_count: u16,
    ) {
        shard
            .runtime_states
            .insert(run, crate::shard::RuntimeState::Resumable);
        let wf = crate::admission::empty_workflow();
        shard.pending_workflows.insert(run, wf.clone());
        shard
            .runs
            .insert(run, Self::build_run_state(frame, wf, slot_count));
    }

    #[cfg(feature = "test-util")]
    fn build_run_state(
        frame: vb_core::frame::RunFrame,
        workflow: vb_core::workflow::CompiledWorkflow,
        slot_count: u16,
    ) -> crate::shard::RunState {
        use crate::primitives::collect::CollectStates;
        crate::shard::RunState {
            frame,
            workflow,
            store: vb_core::value_store::ValueStore::with_max_slots(slot_count),
            action_attempts: Box::new([]),
            admission: None,
            collect_states: CollectStates::default(),
            action_contracts: Box::new([]),
        }
    }

    #[cfg(feature = "test-util")]
    fn insert_timer(
        &mut self,
        run: vb_core::ids::RunId,
        shard_idx: usize,
        mut timer: crate::shard::PendingTimer,
    ) -> RuntimeResult<()> {
        let shard = self
            .shards
            .get_mut(shard_idx)
            .ok_or(RuntimeError::RunNotFound)?;
        let generation = shard
            .next_pending_timer_generation(run)
            .ok_or(RuntimeError::InvalidTimerFire)?;
        timer.generation = generation;
        shard.pending_timer_insert(run, timer);
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
        usize::try_from(remainder).unwrap_or_default()
    }

    fn shard_for(&self, run: RunId) -> Result<&Shard, RuntimeError> {
        self.shards
            .get(self.shard_index(run))
            .ok_or(RuntimeError::RunNotFound)
    }
}
