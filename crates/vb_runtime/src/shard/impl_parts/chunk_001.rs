use crate::shard::bounded_outcomes::BoundedOutcomeIndex;
use crate::shard::lru_ring::LruRing;
use crate::shard::types::{RuntimeState, TerminalOutcome};
use crate::AskAnswer;

impl Shard {
    /// Creates a new shard with the given configuration.
    pub fn new(config: ShardConfig) -> RuntimeResult<Self> {
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
    ) -> RuntimeResult<Self> {
        config.validate()?;
        Ok(Self {
            command_queue: ShardCommandQueue::from_config(config),
            runs: IndexMap::new(),
            runtime_states: IndexMap::new(),
            terminal_runs: LruRing::try_new(config.max_terminal_runs, config.terminal_runs_ttl_ticks)?,
            terminal_outcomes: BoundedOutcomeIndex::with_capacity(config.max_terminal_outcomes),
            journal_sequences: IndexMap::new(),
            pending_timers: IndexMap::new(),
            frame_pools: IndexMap::new(),
            trace_ring: TraceRing::new(config.trace_capacity),
            counters: ShardCounters::new(),
            step_budget_per_tick: config.step_budget_per_tick,
            max_active_runs: config.max_active_runs,
            coalesce_window_ticks: config.coalesce_window_ticks,
            policy: config.policy,
            snapshot_interval_steps: config.snapshot_interval_steps,
            artifact_store,
            inspect_response: None,
            shutting_down: std::sync::atomic::AtomicBool::new(false),
            current_tick: TimerTick::new(0),
            journal,
            admission_lock: std::sync::Mutex::new(()),
            current_coalesce_window_remaining: config.coalesce_window_ticks,
            coalesce_buffer: Vec::with_capacity(
                usize::try_from(config.coalesce_window_ticks).unwrap_or(0_usize),
            ),
            #[cfg(feature = "test-util")]
            pending_workflows: IndexMap::new(),
        })
    }

    /// Creates a new shard with the given configuration and journal sink.
    ///
    /// For storage-backed journals (e.g., `StorageRuntimeJournal`), the shard uses
    /// `StorageArtifactStore` so that strict/journaled admission can validate artifacts
    /// against real durable storage. For noop/volatile strict and journaled journals,
    /// `MissingAcceptedArtifactStore` is used so direct runtime construction without a
    /// storage-backed accepted-artifact source rejects admission instead of silently
    /// accepting unbacked artifacts.
    pub fn new_with_journal(
        config: ShardConfig,
        journal: SharedRuntimeJournal,
    ) -> RuntimeResult<Self> {
        let artifact_store: crate::admission::SharedAcceptedArtifactStore = match config.policy {
            vb_core::policy::RuntimePolicy::Relaxed => {
                crate::admission::AlwaysPresentArtifactStore::shared()
            }
            vb_core::policy::RuntimePolicy::Strict | vb_core::policy::RuntimePolicy::Journaled => {
                if let Some(fjall_journal) = journal.storage_journal() {
                    // Storage-backed strict/journaled runtime validates accepted artifacts
                    // from durable storage before admission.
                    std::sync::Arc::new(crate::admission::StorageArtifactStore::new(fjall_journal))
                } else {
                    crate::admission::MissingAcceptedArtifactStore::shared()
                }
            }
            _ => crate::admission::MissingAcceptedArtifactStore::shared(),
        };
        Self::new_with_journal_and_artifact_store(config, journal, artifact_store)
    }

    /// Enqueues a command. Returns `QueueFull` on overflow.
    /// For submit variants, validates journal health before enqueueing
    /// because handle_submit writes to journal before returning.
    /// Returns `ShutdownInProgress` once the shard has begun shutting down,
    /// except for the `Shutdown` sentinel which is always permitted so the
    /// caller can drive the drain to completion.
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        if self
            .shutting_down
            .load(std::sync::atomic::Ordering::Acquire)
            && !matches!(cmd, ShardCommand::Shutdown)
        {
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

    /// Acquires the admission gate lock for the duration of a preflight+enqueue pair.
    ///
    /// The guard must be held continuously from before `preflight_*` evaluation
    /// to after `ShardCommand::enqueue` so that two concurrent submits targeting
    /// the same shard cannot squeeze in between the preflight and the enqueue,
    /// keeping the budget reservation atomic with the queue commit.
    ///
    /// Poisoned-lock errors are mapped to `RuntimeError::JournalPoisoned` since
    /// the lock lives on the shard's per-state structure and poison can only
    /// happen if a previous holder panicked mid-admission. Production code
    /// never panics, so this is a defense-in-depth typed error path.
    pub(crate) fn lock_admission(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, ()>, crate::RuntimeError> {
        match self.admission_lock.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                // Recover the guard; the lock is still held. We mark this as
                // a journal-poisoned runtime error to avoid introducing a new
                // error variant for a defense-in-depth case the runtime
                // contract never produces.
                drop(poisoned.into_inner());
                Err(crate::RuntimeError::JournalPoisoned)
            }
        }
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
    pub fn runtime_state_insert(&mut self, run_id: RunId, state: RuntimeState) {
        self.runtime_states.insert(run_id, state);
    }

    /// Returns true if the given run ID is in the terminal state.
    #[must_use]
    pub fn terminal_runs_contains(&self, run_id: RunId) -> bool {
        self.terminal_runs.contains(&run_id)
    }

    /// Inserts a run state for the given run ID.
    pub fn run_state_insert(&mut self, run_id: RunId, state: RunState) {
        self.runs.insert(run_id, state);
    }

    /// Inserts a run into the bounded terminal-runs LRU ring (MEM-01).
    ///
    /// The legacy `()`-returning contract is preserved: callers from
    /// `finish_run`, `fail_run_state`, `handle_cancel`, and `handle_kill`
    /// rely on the side effect of membership, not on a typed error. When
    /// the ring is at capacity, the entry is force-inserted (the ring
    /// grows past capacity) and `LruRingCounters::capacity_overflows`
    /// is incremented. The entry is therefore never silently dropped;
    /// the overflow is observable via `terminal_runs_counters()`.
    pub fn terminal_runs_insert(&mut self, run_id: RunId) {
        let now = self.current_tick;
        self.terminal_runs.force_insert(run_id, now);
    }

    /// Strict terminal-runs insert: returns `RuntimeError::TerminalRunsLruFull`
    /// when the ring is at capacity and the entry cannot be admitted.
    ///
    /// Use this from new call sites that want the LRU to refuse the
    /// insert rather than force-grow the ring.
    pub fn terminal_runs_try_insert(&mut self, run_id: RunId) -> RuntimeResult<()> {
        let now = self.current_tick;
        self.terminal_runs.insert(run_id, now)
    }

    /// Returns a snapshot of the LRU ring's diagnostic counters.
    #[must_use]
    pub const fn terminal_runs_counters(&self) -> crate::shard::lru_ring::LruRingCounters {
        self.terminal_runs.counters()
    }

    /// Records the terminal outcome for a run that is being moved to the
    /// terminal set. Idempotent: a later call for the same run id replaces
    /// the prior outcome without consuming additional capacity. When the
    /// bounded map is at capacity, the oldest entry is evicted FIFO before
    /// the new outcome is recorded (RQ-W0-10).
    pub fn terminal_outcome_record(&mut self, run_id: RunId, outcome: TerminalOutcome) {
        self.terminal_outcomes.record(run_id, outcome);
    }

    /// Returns the recorded terminal outcome for a run id, if any.
    #[must_use]
    pub fn terminal_outcome_get(&self, run_id: RunId) -> Option<TerminalOutcome> {
        self.terminal_outcomes.get(run_id)
    }

    /// Removes the recorded terminal outcome for a run id, if any.
    ///
    /// Returns `true` if an entry was removed.
    pub fn terminal_outcomes_remove(&mut self, run_id: RunId) -> bool {
        self.terminal_outcomes.remove(run_id)
    }

    /// Removes a run from the terminal runs set.
    ///
    /// Returns the typed `RuntimeError::Core { InternalInvariantViolation }`
    /// if the underlying [`LruRing`] exposes an internal corruption
    /// (e.g. a live slot is missing from the arena, or the doubly-linked
    /// list pointers reference a free slot). Production callers MUST
    /// propagate this error; the corruption cannot be repaired silently.
    pub fn terminal_runs_remove(
        &mut self,
        run_id: RunId,
    ) -> Result<(), crate::RuntimeError> {
        self.terminal_runs
            .remove(&run_id)
            .map_err(crate::shard::lru_ring::LruRingError::into_runtime_error)
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
        if self.terminal_runs_contains(run) {
            return match self.terminal_outcome_get(run) {
                Some(outcome) => InspectResponse::Terminal {
                    run,
                    correlation,
                    outcome,
                },
                // The run is in the terminal set but the outcome record is missing.
                // Surface this as an explicit `Tombstoned` rather than silently
                // synthesizing `Failed`; the caller decides how to handle the
                // inconsistency instead of inheriting a fabricated outcome.
                None => InspectResponse::Tombstoned { run, correlation },
            };
        }
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
    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Returns a read-only status snapshot without draining queues or mutating shard state.
    #[must_use]
    pub fn status(&self) -> ShardStatus {
        let shutting_down = self.is_shutting_down();
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
            snapshot_interval_steps: self.snapshot_interval_steps,
        }
    }
}
