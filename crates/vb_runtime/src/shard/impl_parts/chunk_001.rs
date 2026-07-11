use crate::shard::types::{RunAggregate, RuntimeState};

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
            run_aggregate: RunAggregate::new(),
            journal_sequences: IndexMap::new(),
            accounted_executed_steps: IndexMap::new(),
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
        self.runs
            .len()
            .saturating_add(self.run_aggregate.checked_out_len())
    }

    fn run_capacity_error(capacity: usize) -> RuntimeError {
        RuntimeError::ActiveRunCapacityExceeded { capacity }
    }

    pub(crate) fn prepare_run_slots(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.reserve_run_state_slot(run_id)?;
        self.reserve_checked_out_run_slot(run_id)?;
        self.reserve_runtime_state_slot(run_id)?;
        self.reserve_journal_sequence_slot(run_id)?;
        self.reserve_pending_timer_slot(run_id)?;
        self.reserve_pending_action_slot(run_id)
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

    fn reserve_run_state_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        Self::reserve_index_map_slot(&mut self.runs, run_id, self.max_active_runs)
    }

    pub(crate) fn reserve_checked_out_run_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .reserve_checked_out_slot(run_id, self.max_active_runs)
    }

    fn reserve_runtime_state_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .reserve_runtime_state_slot(run_id, self.max_active_runs)
    }

    fn reserve_journal_sequence_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        Self::reserve_index_map_slot(&mut self.journal_sequences, run_id, self.max_active_runs)
    }

    pub(crate) fn reserve_pending_timer_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .reserve_pending_timer_slot(run_id, self.max_active_runs)
    }

    fn reserve_pending_action_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .reserve_pending_action_slot(run_id, self.max_active_runs)
    }

    #[cfg(not(kani))]
    pub(crate) fn append_journal_event(&mut self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let run = event.run_id();
        let seq = self.journal_sequence_for(run);
        self.journal.append_sequenced(event, seq)?;
        self.advance_journal_sequence(run, seq)
    }

    #[cfg(not(kani))]
    pub(crate) fn append_journal_event_batch(
        &mut self,
        events: &[RuntimeJournalEvent],
    ) -> RuntimeResult<()> {
        let Some((first, rest)) = events.split_first() else {
            return Ok(());
        };
        let run = first.run_id();
        for event in rest {
            if event.run_id() != run {
                return Err(RuntimeError::UnsupportedOperation {
                    operation: "mixed_run_journal_batch",
                });
            }
        }
        let seq = self.journal_sequence_for(run);
        let next = Self::journal_sequence_after(seq, events.len())?;
        self.reserve_journal_sequence_slot(run)?;
        self.journal.append_sequenced_batch(events, seq)?;
        let _previous = self.journal_sequences.insert(run, next);
        Ok(())
    }

    #[cfg(not(kani))]
    pub(crate) fn append_journal_events_atomically<const N: usize>(
        &mut self,
        events: [RuntimeJournalEvent; N],
    ) -> RuntimeResult<()> {
        self.append_journal_event_batch(&events)
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

    #[cfg(kani)]
    pub(crate) fn append_journal_event_batch(
        &mut self,
        _events: &[RuntimeJournalEvent],
    ) -> RuntimeResult<()> {
        Ok(())
    }

    #[cfg(kani)]
    pub(crate) fn append_journal_events_atomically<const N: usize>(
        &mut self,
        _events: [RuntimeJournalEvent; N],
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

    #[cfg(not(kani))]
    fn journal_sequence_after(seq: EventSeq, count: usize) -> RuntimeResult<EventSeq> {
        let count = u64::try_from(count)
            .map_err(|_| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
        seq.get()
            .checked_add(count)
            .map(EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))
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
        self.run_aggregate.pending_timer_len()
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

    /// Iterates active run state without exposing direct map mutation.
    pub(crate) fn active_runs_iter(&self) -> impl Iterator<Item = (&RunId, &RunState)> {
        self.runs.iter()
    }

    /// Returns true if a run with the given ID exists.
    #[must_use]
    pub fn run_state_contains(&self, run_id: RunId) -> bool {
        self.runs.contains_key(&run_id)
    }

    /// Removes and returns the run state for the given run ID.
    pub(crate) fn run_state_remove(&mut self, run_id: RunId) -> Option<RunState> {
        let state = self.runs.swap_remove(&run_id)?;
        if !self.checked_out_run_contains(run_id) {
            self.runtime_state_remove(run_id);
            let _removed_timer = self.pending_timer_remove(run_id);
            let _removed_action = self.pending_action_remove(run_id);
        }
        Some(state)
    }

    /// Returns the runtime state for the given run ID, if it exists.
    #[must_use]
    pub fn runtime_state_get(&self, run_id: RunId) -> Option<RuntimeState> {
        self.run_aggregate.runtime_state_get(run_id)
    }

    /// Inserts a non-terminal runtime state for the given run ID.
    pub(crate) fn runtime_state_insert(
        &mut self,
        run_id: RunId,
        state: RuntimeState,
    ) -> RuntimeResult<Option<RuntimeState>> {
        self.run_aggregate
            .runtime_state_insert(&self.runs, run_id, state, self.max_active_runs)
    }

    /// Removes the runtime state for the given run ID, if it exists.
    pub(crate) fn runtime_state_remove(&mut self, run_id: RunId) {
        if self.run_state_contains(run_id) || self.checked_out_run_contains(run_id) {
            return;
        }
        self.run_aggregate.runtime_state_remove(run_id);
    }

    /// Clears runtime state for a terminal event apply path, including when
    /// the run is checked out of `runs` for deterministic drive cleanup.
    pub(crate) fn runtime_state_terminal_clear(&mut self, run_id: RunId) {
        self.run_aggregate.runtime_state_terminal_clear(run_id);
    }

    /// Returns true if the given run ID is in the terminal state.
    #[must_use]
    pub fn terminal_runs_contains(&self, run_id: RunId) -> bool {
        self.run_aggregate.terminal_contains(run_id)
    }

    /// Inserts a run state for the given run ID.
    pub(crate) fn run_state_insert(
        &mut self,
        run_id: RunId,
        state: RunState,
    ) -> RuntimeResult<Option<RunState>> {
        if self.terminal_runs_contains(run_id) || self.run_state_contains(run_id) {
            return Err(RuntimeError::RunAlreadyExists);
        }
        if !self.run_state_has_aggregate_visibility(run_id) {
            return Err(RuntimeError::RunNotFound);
        }
        self.reserve_run_state_slot(run_id)?;
        let previous = self.runs.insert(run_id, state);
        self.run_aggregate.checked_out_remove(run_id);
        Ok(previous)
    }

    fn run_state_has_aggregate_visibility(&self, run_id: RunId) -> bool {
        self.runtime_state_get(run_id).is_some() || self.checked_out_run_contains(run_id)
    }

    pub(crate) fn admit_run_state(
        &mut self,
        run_id: RunId,
        state: RunState,
        runtime_state: RuntimeState,
    ) -> RuntimeResult<()> {
        self.validate_new_active_run(run_id, runtime_state)?;
        self.reserve_run_state_slot(run_id)?;
        self.reserve_runtime_state_slot(run_id)?;
        let _previous = self.runs.insert(run_id, state);
        match self.runtime_state_insert(run_id, runtime_state) {
            Ok(_) => Ok(()),
            Err(error) => {
                let _removed = self.runs.swap_remove(&run_id);
                Err(error)
            }
        }
    }

    fn validate_new_active_run(
        &self,
        run_id: RunId,
        runtime_state: RuntimeState,
    ) -> RuntimeResult<()> {
        if runtime_state == RuntimeState::Failed {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "runtime_state_failed_terminal_split",
            });
        }
        if self.run_state_contains(run_id)
            || self.checked_out_run_contains(run_id)
            || self.runtime_state_get(run_id).is_some()
            || self.terminal_runs_contains(run_id)
        {
            return Err(RuntimeError::RunAlreadyExists);
        }
        Ok(())
    }

    pub(crate) fn checked_out_run_insert(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .checked_out_insert(run_id, self.max_active_runs)
    }

    pub(crate) fn checked_out_run_remove(&mut self, run_id: RunId) {
        self.run_aggregate.checked_out_remove(run_id);
    }

    pub(crate) fn checked_out_run_contains(&self, run_id: RunId) -> bool {
        self.run_aggregate.checked_out_contains(run_id)
    }

    pub(crate) fn checked_out_run_iter(&self) -> impl Iterator<Item = &RunId> {
        self.run_aggregate.checked_out_iter()
    }

    /// Inserts a run into the terminal runs set.
    pub fn terminal_runs_insert(&mut self, run_id: RunId) -> RuntimeResult<bool> {
        self.run_aggregate
            .terminal_insert(&self.runs, run_id, self.max_active_runs)
    }

    pub(crate) fn reserve_terminal_run_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .reserve_terminal_insert_slot(run_id, self.max_active_runs)
    }

    /// Removes a run from the terminal runs set.
    pub fn terminal_runs_remove(&mut self, run_id: RunId) {
        self.run_aggregate.terminal_remove(run_id);
    }

    /// Inserts a pending timer for the given run ID.
    pub fn pending_timer_insert(
        &mut self,
        run_id: RunId,
        timer: PendingTimer,
    ) -> RuntimeResult<Option<PendingTimer>> {
        self.run_aggregate.pending_timer_insert(
            &self.runs,
            self.max_active_runs,
            run_id,
            timer,
        )
    }

    /// Returns the pending timer for the given run ID, if it exists.
    #[must_use]
    pub fn pending_timer_get(&self, run_id: RunId) -> Option<PendingTimer> {
        self.run_aggregate.pending_timer_get(run_id)
    }

    /// Returns a clone of all pending timers.
    #[must_use]
    pub fn pending_timer_clone(&self) -> IndexMap<RunId, PendingTimer> {
        self.run_aggregate.pending_timer_clone()
    }

    /// Removes and returns the pending timer for the given run ID.
    pub fn pending_timer_remove(&mut self, run_id: RunId) -> Option<PendingTimer> {
        self.run_aggregate.pending_timer_remove(run_id)
    }

    /// Returns true if a pending timer exists for the given run ID.
    #[must_use]
    pub fn pending_timer_contains(&self, run_id: RunId) -> bool {
        self.run_aggregate.pending_timer_contains(run_id)
    }

    pub(crate) fn pending_timer_iter(&self) -> impl Iterator<Item = (&RunId, &PendingTimer)> {
        self.run_aggregate.pending_timer_iter()
    }

    pub(crate) fn clear_pending_timers(&mut self) {
        self.run_aggregate.pending_timer_clear();
    }

    /// Inserts an in-flight action ticket for the given run ID.
    pub fn pending_action_insert(
        &mut self,
        run_id: RunId,
        ticket: vb_core::action::ActionTicket,
    ) -> RuntimeResult<Option<vb_core::action::ActionTicket>> {
        self.run_aggregate.pending_action_insert(
            &self.runs,
            self.max_active_runs,
            run_id,
            ticket,
        )
    }

    /// Returns the in-flight action ticket for the given run, if any.
    #[must_use]
    pub fn pending_action_get(
        &self,
        run_id: RunId,
    ) -> Option<vb_core::action::ActionTicket> {
        self.run_aggregate.pending_action_get(run_id)
    }

    #[must_use]
    pub(crate) fn pending_action_len(&self) -> usize {
        self.run_aggregate.pending_action_len()
    }

    /// Returns a clone of all pending action tickets.
    #[must_use]
    pub fn pending_action_clone(&self) -> IndexMap<RunId, vb_core::action::ActionTicket> {
        self.run_aggregate.pending_action_clone()
    }

    /// Removes and returns the in-flight action ticket for the given
    /// run ID.
    pub fn pending_action_remove(
        &mut self,
        run_id: RunId,
    ) -> Option<vb_core::action::ActionTicket> {
        self.run_aggregate.pending_action_remove(run_id)
    }

    pub(crate) fn pending_action_iter(
        &self,
    ) -> impl Iterator<Item = (&RunId, &vb_core::action::ActionTicket)> {
        self.run_aggregate.pending_action_iter()
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
            ShardCommand::Recover {
                run,
                frame,
                workflow_digest,
            } => self.handle_recover(run, frame, workflow_digest)?,
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
    pub(crate) fn prepare_evidence_events(
        &self,
        run: RunId,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeResult<(Vec<RuntimeJournalEvent>, Vec<TraceEvent>)> {
        let event_count = evidence.len();
        let mut journal_events = Vec::new();
        Self::reserve_drive_vec(&mut journal_events, event_count)?;
        let mut trace_events = Vec::new();
        Self::reserve_drive_vec(&mut trace_events, event_count)?;
        for event in evidence.drain() {
            Self::prepare_evidence_event(run, event, &mut journal_events, &mut trace_events)?;
        }
        Ok((journal_events, trace_events))
    }

    fn reserve_drive_vec<T>(items: &mut Vec<T>, additional: usize) -> RuntimeResult<()> {
        items
            .try_reserve(additional)
            .map_err(|_| RuntimeError::from(vb_storage::JournalError::QueueFull))
    }

    pub(crate) fn push_drive_journal_event(
        events: &mut Vec<RuntimeJournalEvent>,
        event: RuntimeJournalEvent,
    ) -> RuntimeResult<()> {
        Self::reserve_drive_vec(events, 1)?;
        events.push(event);
        Ok(())
    }

    fn prepare_evidence_event(
        run: RunId,
        event: EvidenceEvent,
        journal_events: &mut Vec<RuntimeJournalEvent>,
        trace_events: &mut Vec<TraceEvent>,
    ) -> RuntimeResult<()> {
        match event {
            EvidenceEvent::StepStarted { step } => {
                Self::push_drive_journal_event(
                    journal_events,
                    RuntimeJournalEvent::StepStarted { run, step },
                )?;
                trace_events.push(TraceEvent::StepStarted { run, step });
                Ok(())
            }
            EvidenceEvent::StepSucceeded { step, output } => {
                Self::push_drive_journal_event(
                    journal_events,
                    RuntimeJournalEvent::StepSucceeded {
                        run,
                        step,
                        output: match output {
                            Some(slot) => slot,
                            None => SlotIdx::ZERO,
                        },
                        attempt: 1,
                    },
                )
            }
            EvidenceEvent::SlotWritten {
                slot,
                value,
                taint,
                extra,
            } => Self::prepare_slot_written_event(
                run,
                slot,
                value,
                taint,
                extra,
                journal_events,
                trace_events,
            ),
        }
    }

    fn prepare_slot_written_event(
        run: RunId,
        slot: SlotIdx,
        value: vb_core::value::SlotValue,
        taint: vb_core::Taint,
        extra: Option<crate::primitives::collect::CollectPaginationState>,
        journal_events: &mut Vec<RuntimeJournalEvent>,
        trace_events: &mut Vec<TraceEvent>,
    ) -> RuntimeResult<()> {
        let encoded = postcard::to_allocvec(&value).map_err(|_| RuntimeError::EncodeFailed)?;
        let encoded_extra = extra
            .map(|state| postcard::to_allocvec(&state))
            .transpose()
            .map_err(|_| RuntimeError::EncodeFailed)?;
        trace_events.push(TraceEvent::SlotWritten {
            run,
            slot,
            value: encoded.clone(),
        });
        Self::push_drive_journal_event(
            journal_events,
            RuntimeJournalEvent::SlotWritten {
                run,
                slot,
                value: encoded,
                taint,
                extra: encoded_extra,
            },
        )
    }

    pub(crate) fn push_trace_events(&mut self, trace_events: Vec<TraceEvent>) {
        for event in trace_events {
            self.trace_ring.push(event);
        }
    }
}
