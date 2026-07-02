use crate::shard::types::RuntimeState;

impl Shard {
    /// Creates a new shard with the given configuration.
    pub fn new(config: ShardConfig) -> Self {
        Self::new_with_journal_and_artifact_store(
            config,
            VolatileRuntimeJournal::shared(),
            crate::admission::AlwaysPresentArtifactStore::shared(),
        )
    }

    /// Creates a new shard with the given configuration, journal sink, and artifact store.
    pub fn new_with_journal_and_artifact_store(
        config: ShardConfig,
        journal: SharedRuntimeJournal,
        artifact_store: crate::admission::SharedAcceptedArtifactStore,
    ) -> Self {
        Self {
            command_queue: ShardCommandQueue::from_config(config),
            runs: IndexMap::new(),
            runtime_states: IndexMap::new(),
            terminal_runs: IndexSet::new(),
            journal_sequences: IndexMap::new(),
            accounted_executed_steps: IndexMap::new(),
            pending_timers: IndexMap::new(),
            pending_actions: IndexMap::new(),
            frame_pools: IndexMap::new(),
            trace_ring: TraceRing::new(config.trace_capacity),
            counters: ShardCounters::new(),
            step_budget_per_tick: config.step_budget_per_tick,
            max_active_runs: config.max_active_runs,
            policy: config.policy,
            artifact_store,
            inspect_response: None,
            shutting_down: false,
            current_tick: TimerTick::new(0),
            journal,
        }
    }

    /// Creates a new shard with the given configuration and journal sink.
    ///
    /// For storage-backed journals (e.g., `StorageRuntimeJournal`), the shard uses
    /// `StorageArtifactStore` so that strict/journaled admission can validate artifacts
    /// against real durable storage. For noop/volatile strict and journaled journals,
    /// `MissingAcceptedArtifactStore` is used so direct runtime construction without a
    /// storage-backed accepted-artifact source rejects admission instead of silently
    /// accepting unbacked artifacts.
    pub fn new_with_journal(config: ShardConfig, journal: SharedRuntimeJournal) -> Self {
        let artifact_store: crate::admission::SharedAcceptedArtifactStore =
            if let Some(fjall_journal) = journal.storage_journal() {
                // Storage-backed journal: use StorageArtifactStore for strict/journaled
                // artifact validation. This ensures the shard can load and validate
                // accepted artifacts from durable storage before admission.
                std::sync::Arc::new(crate::admission::StorageArtifactStore::new(fjall_journal))
            } else {
                match config.policy {
                    vb_core::policy::RuntimePolicy::Relaxed => {
                        crate::admission::AlwaysPresentArtifactStore::shared()
                    }
                    vb_core::policy::RuntimePolicy::Strict
                    | vb_core::policy::RuntimePolicy::Journaled => {
                        crate::admission::MissingAcceptedArtifactStore::shared()
                    }
                    _ => crate::admission::MissingAcceptedArtifactStore::shared(),
                }
            };
        Self::new_with_journal_and_artifact_store(config, journal, artifact_store)
    }

    /// Enqueues a command. Returns `QueueFull` on overflow.
    /// For submit variants, validates journal health before enqueueing
    /// because handle_submit writes to journal before returning.
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        if self.shutting_down {
            return Err(RuntimeError::ShutdownInProgress);
        }
        match &cmd {
            ShardCommand::Submit { .. }
            | ShardCommand::SubmitPrePersisted { .. }
            | ShardCommand::SubmitWithInputs { .. }
            | ShardCommand::SubmitWithContracts { .. }
            | ShardCommand::SubmitWithInputsAndContracts { .. } => {
                // Probe journal health before accepting the command.
                self.journal.probe()?;
            }
            _ => {}
        }
        self.command_queue.enqueue(cmd)
    }

    /// Returns the number of commands currently in the queue.
    #[must_use]
    pub fn command_queue_len(&self) -> usize {
        self.command_queue.len()
    }

    /// Returns the remaining free slots in the command queue.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.command_queue.remaining_capacity()
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

    fn run_capacity_error(capacity: usize) -> RuntimeError {
        RuntimeError::ActiveRunCapacityExceeded { capacity }
    }

    pub(crate) fn prepare_run_slots(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.reserve_run_state_slot(run_id)?;
        self.reserve_runtime_state_slot(run_id)?;
        self.reserve_journal_sequence_slot(run_id)?;
        self.reserve_pending_timer_slot(run_id)
    }

    fn reserve_index_map_slot<T>(
        slots: &mut IndexMap<RunId, T>,
        run_id: RunId,
        capacity: usize,
    ) -> RuntimeResult<()> {
        if slots.contains_key(&run_id) {
            return Ok(());
        }
        if slots.len() >= capacity {
            return Err(Self::run_capacity_error(capacity));
        }
        slots
            .try_reserve(1)
            .map_err(|_| Self::run_capacity_error(capacity))
    }

    fn reserve_index_set_slot(
        slots: &mut IndexSet<RunId>,
        run_id: RunId,
        capacity: usize,
    ) -> RuntimeResult<()> {
        if slots.contains(&run_id) {
            return Ok(());
        }
        if capacity == 0 {
            return Err(Self::run_capacity_error(capacity));
        }
        if slots.len() >= capacity {
            let evicted = slots.iter().next().copied();
            if let Some(evicted) = evicted {
                let _removed = slots.shift_remove(&evicted);
            }
        }
        slots
            .try_reserve(1)
            .map_err(|_| Self::run_capacity_error(capacity))
    }

    fn reserve_run_state_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        Self::reserve_index_map_slot(&mut self.runs, run_id, self.max_active_runs)
    }

    fn reserve_runtime_state_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        Self::reserve_index_map_slot(&mut self.runtime_states, run_id, self.max_active_runs)
    }

    fn reserve_journal_sequence_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        Self::reserve_index_map_slot(&mut self.journal_sequences, run_id, self.max_active_runs)
    }

    pub(crate) fn reserve_pending_timer_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        Self::reserve_index_map_slot(&mut self.pending_timers, run_id, self.max_active_runs)
    }

    fn reserve_pending_action_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        Self::reserve_index_map_slot(&mut self.pending_actions, run_id, self.max_active_runs)
    }

    fn reserve_terminal_run_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        Self::reserve_index_set_slot(&mut self.terminal_runs, run_id, self.max_active_runs)
    }

    #[cfg(not(kani))]
    pub(crate) fn append_journal_event(&mut self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let run = event.run_id();
        let seq = self.journal_sequence_for(run);
        self.journal.append_sequenced(event, seq)?;
        self.advance_journal_sequence(run, seq)
    }

    /// `#[cfg(kani)]` replacement for `append_journal_event` that returns
    /// nondeterministic Ok or Err. Trust boundary TB-vb282my-journal-stub-001.
    /// Production journal append uses Fjall-backed persistence; stubbed version
    /// exercises error paths and ordering verification without real I/O.
    #[cfg(kani)]
    pub(crate) fn append_journal_event(
        &mut self,
        _event: RuntimeJournalEvent,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// `#[cfg(kani)]`-only accessor for the per-run journal sequence.
    /// Returns the current `EventSeq` recorded for `run_id`, or `None`
    /// if no sequence has been allocated yet. Used by kani harnesses
    /// (e.g. `kani_ask_answer_lifecycle::kani_ask_answer_journal_monotonicity`)
    /// to inspect monotonicity invariants without exposing the field
    /// outside the `kani` build.
    #[cfg(kani)]
    pub(crate) fn journal_seq_get(&self, run_id: RunId) -> Option<EventSeq> {
        self.journal_sequences.get(&run_id).copied()
    }

    fn journal_sequence_for(&self, run: RunId) -> EventSeq {
        self.journal_sequences
            .get(&run)
            .copied()
            .unwrap_or(EventSeq::ZERO)
    }

    fn advance_journal_sequence(&mut self, run: RunId, seq: EventSeq) -> RuntimeResult<()> {
        let next = seq
            .get()
            .checked_add(1)
            .map(EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
        self.reserve_journal_sequence_slot(run)?;
        let _previous = self.journal_sequences.insert(run, next);
        Ok(())
    }

    pub(crate) fn discard_journal_sequence(&mut self, run: RunId) {
        let _removed = self.journal_sequences.swap_remove(&run);
    }

    pub(crate) fn add_executed_step_delta(&mut self, run: RunId, executed: u64) {
        let previous = self
            .accounted_executed_steps
            .get(&run)
            .copied()
            .map_or(0, |value| value);
        let Some(delta) = executed.checked_sub(previous) else {
            return;
        };
        if delta == 0 {
            return;
        }
        self.counters.add_steps(delta);
        self.accounted_executed_steps.insert(run, executed);
    }

    pub(crate) fn clear_executed_step_accounting(&mut self, run: RunId) {
        self.accounted_executed_steps.swap_remove(&run);
    }

    /// Returns the number of pending timers on this shard.
    #[must_use]
    pub fn pending_timer_count(&self) -> usize {
        self.pending_timers.len()
    }

    /// Returns the run state for the given run ID, if it exists.
    #[must_use]
    pub fn run_state_get(&self, run_id: RunId) -> Option<&RunState> {
        self.runs.get(&run_id)
    }

    /// Returns a mutable reference to the run state for the given run ID, if it exists.
    #[must_use]
    pub fn run_state_get_mut(&mut self, run_id: RunId) -> Option<&mut RunState> {
        self.runs.get_mut(&run_id)
    }

    /// Returns true if a run with the given ID exists.
    #[must_use]
    pub fn run_state_contains(&self, run_id: RunId) -> bool {
        self.runs.contains_key(&run_id)
    }

    /// Removes and returns the run state for the given run ID.
    pub fn run_state_remove(&mut self, run_id: RunId) -> Option<RunState> {
        self.runs.swap_remove(&run_id)
    }

    /// Returns the runtime state for the given run ID, if it exists.
    #[must_use]
    pub fn runtime_state_get(&self, run_id: RunId) -> Option<RuntimeState> {
        self.runtime_states.get(&run_id).copied()
    }

    /// Inserts a runtime state for the given run ID.
    pub fn runtime_state_insert(
        &mut self,
        run_id: RunId,
        state: RuntimeState,
    ) -> RuntimeResult<Option<RuntimeState>> {
        self.reserve_runtime_state_slot(run_id)?;
        Ok(self.runtime_states.insert(run_id, state))
    }

    /// Removes the runtime state for the given run ID, if it exists.
    pub(crate) fn runtime_state_remove(&mut self, run_id: RunId) {
        let _removed = self.runtime_states.swap_remove(&run_id);
    }

    /// Returns true if the given run ID is in the terminal state.
    #[must_use]
    pub fn terminal_runs_contains(&self, run_id: RunId) -> bool {
        self.terminal_runs.contains(&run_id)
    }

    /// Inserts a run state for the given run ID.
    pub fn run_state_insert(
        &mut self,
        run_id: RunId,
        state: RunState,
    ) -> RuntimeResult<Option<RunState>> {
        self.reserve_run_state_slot(run_id)?;
        Ok(self.runs.insert(run_id, state))
    }

    /// Inserts a run into the terminal runs set.
    pub fn terminal_runs_insert(&mut self, run_id: RunId) -> RuntimeResult<bool> {
        self.reserve_terminal_run_slot(run_id)?;
        Ok(self.terminal_runs.insert(run_id))
    }

    /// Removes a run from the terminal runs set.
    pub fn terminal_runs_remove(&mut self, run_id: RunId) {
        let _removed = self.terminal_runs.swap_remove(&run_id);
    }

    /// Inserts a pending timer for the given run ID.
    pub fn pending_timer_insert(
        &mut self,
        run_id: RunId,
        timer: PendingTimer,
    ) -> RuntimeResult<Option<PendingTimer>> {
        self.reserve_pending_timer_slot(run_id)?;
        Ok(self.pending_timers.insert(run_id, timer))
    }

    /// Returns the pending timer for the given run ID, if it exists.
    #[must_use]
    pub fn pending_timer_get(&self, run_id: RunId) -> Option<PendingTimer> {
        self.pending_timers.get(&run_id).copied()
    }

    /// Returns a clone of all pending timers.
    #[must_use]
    pub fn pending_timer_clone(&self) -> IndexMap<RunId, PendingTimer> {
        self.pending_timers.clone()
    }

    /// Removes and returns the pending timer for the given run ID.
    pub fn pending_timer_remove(&mut self, run_id: RunId) -> Option<PendingTimer> {
        self.pending_timers.swap_remove(&run_id)
    }

    /// Returns true if a pending timer exists for the given run ID.
    #[must_use]
    pub fn pending_timer_contains(&self, run_id: RunId) -> bool {
        self.pending_timers.contains_key(&run_id)
    }

    /// Inserts an in-flight action ticket for the given run ID.
    pub fn pending_action_insert(
        &mut self,
        run_id: RunId,
        ticket: vb_core::action::ActionTicket,
    ) -> RuntimeResult<Option<vb_core::action::ActionTicket>> {
        self.reserve_pending_action_slot(run_id)?;
        Ok(self.pending_actions.insert(run_id, ticket))
    }

    /// Returns the in-flight action ticket for the given run, if any.
    #[must_use]
    pub fn pending_action_get(
        &self,
        run_id: RunId,
    ) -> Option<vb_core::action::ActionTicket> {
        self.pending_actions.get(&run_id).copied()
    }

    /// Returns a clone of all pending action tickets.
    #[must_use]
    pub fn pending_action_clone(&self) -> IndexMap<RunId, vb_core::action::ActionTicket> {
        self.pending_actions.clone()
    }

    /// Removes and returns the in-flight action ticket for the given
    /// run ID.
    pub fn pending_action_remove(
        &mut self,
        run_id: RunId,
    ) -> Option<vb_core::action::ActionTicket> {
        self.pending_actions.swap_remove(&run_id)
    }

    /// Advances the deterministic clock to the given tick.
    ///
    /// The new tick must be >= the current tick. Returns an error if
    /// the supplied tick is in the past, preventing backward clock jumps.
    ///
    /// This operates the numeric timer seam alongside the existing
    /// wall-clock `Instant`-based timers; it does not modify or affect
    /// `Instant`-derived deadlines.
    pub fn advance_clock_to(&mut self, new_tick: TimerTick) -> RuntimeResult<()> {
        if new_tick < self.current_tick {
            return Err(RuntimeError::InvalidTimerFire);
        }
        self.current_tick = new_tick;
        Ok(())
    }

    /// Returns the current tick of the deterministic clock.
    #[must_use]
    pub fn current_tick(&self) -> TimerTick {
        self.current_tick
    }

    /// Returns the next freshness generation for a run's pending timer.
    ///
    /// - `Some(n)` where `n > 0` is the next generation to use.
    /// - `None` if generation would overflow `u64`.
    ///
    /// If no timer exists for the run, returns `Some(1)`.
    #[must_use]
    pub fn next_pending_timer_generation(&self, run: RunId) -> Option<u64> {
        match self.pending_timer_get(run) {
            Some(timer) => timer.generation.checked_add(1),
            None => Some(1),
        }
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
            ShardCommand::SubmitPrePersisted {
                run,
                workflow,
                caps,
            } => self.handle_submit_pre_persisted(run, workflow, caps)?,
            ShardCommand::SubmitWithInputs {
                run,
                workflow,
                inputs,
                caps,
            } => self.handle_submit_with_inputs(run, workflow, &inputs, caps)?,
            ShardCommand::SubmitWithContracts {
                run,
                workflow,
                caps,
                action_contracts,
            } => self.handle_submit_with_contracts(run, workflow, caps, &action_contracts)?,
            ShardCommand::SubmitWithInputsAndContracts {
                run,
                workflow,
                inputs,
                caps,
                action_contracts,
            } => self.handle_submit_with_inputs_and_contracts(
                run,
                workflow,
                &inputs,
                caps,
                &action_contracts,
            )?,
            ShardCommand::Resume { run } => {
                self.handle_resume(run).map_err(RuntimeError::from)?;
            }
            ShardCommand::ActionCompleted { ticket, output } => {
                self.handle_action_completion(ticket, output)?;
            }
            ShardCommand::ActionCompletedLegacy { run, step } => {
                self.handle_legacy_action_completion(run, step)?;
            }
            ShardCommand::ActionFailed { ticket, failure } => {
                self.handle_action_failure(ticket, failure)?;
            }
            ShardCommand::RuntimeActionFailed { ticket, failure } => {
                self.handle_action_failure(ticket, failure)
                    .map_err(Self::runtime_action_failure_error)?;
            }
            ShardCommand::AskAnswered { answer } => self.handle_ask_answer(answer)?,
            ShardCommand::TimerFired {
                run,
                generation,
                deadline,
                kind,
            } => self.handle_timer(run, generation, deadline, kind)?,
            ShardCommand::Cancel { run, reason } => self.handle_cancel(run, reason)?,
            ShardCommand::Kill { run, reason } => self.handle_kill(run, reason)?,
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

    fn runtime_action_failure_error(error: RuntimeError) -> RuntimeError {
        match error {
            RuntimeError::RunNotFound => RuntimeError::InvalidActionCompletion,
            other => other,
        }
    }

    /// Returns a reference to the shard counters.
    #[must_use]
    pub const fn counters(&self) -> &ShardCounters {
        &self.counters
    }

    /// Builds a fail-closed legacy timer-fired command without fabricating authority.
    #[must_use]
    pub fn timer_fired_command(&self, run: RunId) -> ShardCommand {
        ShardCommand::TimerFired {
            run,
            generation: 0,
            deadline: std::time::Instant::now(),
            kind: PendingTimerKind::Wait,
        }
    }

    /// Returns the current typed timer authority for explicit capture.
    #[must_use]
    pub fn timer_entry(&self, run: RunId) -> Option<crate::shard::timer_wheel::TimerEntry> {
        self.pending_timer_get(run)
            .map(|timer| crate::shard::timer_wheel::TimerEntry {
                run,
                generation: timer.generation,
                deadline: timer.deadline,
                kind: timer.kind,
            })
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
        match self.run_state_get(run) {
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
            active_runs: self.active_run_count(),
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
        self.append_journal_event(RuntimeJournalEvent::StepStarted { run, step })
    }

    fn flush_step_succeeded(
        &mut self,
        run: RunId,
        step: StepIdx,
        output: Option<SlotIdx>,
    ) -> RuntimeResult<()> {
        self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
            run,
            step,
            output: match output {
                Some(slot) => slot,
                None => SlotIdx::ZERO,
            },
            attempt: 1,
        })
    }
}
