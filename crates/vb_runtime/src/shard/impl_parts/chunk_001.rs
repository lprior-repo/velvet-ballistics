use indexmap::IndexSet;
use crate::shard::types::RuntimeState;
use crate::AskAnswer;

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
            current_tick: TimerTick::new(0),
            journal,
            pending_workflows: IndexMap::new(),
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
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
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

    /// Inserts a run into the terminal runs set.
    pub fn terminal_runs_insert(&mut self, run_id: RunId) {
        self.terminal_runs.insert(run_id);
    }

    /// Removes a run from the terminal runs set.
    pub fn terminal_runs_remove(&mut self, run_id: RunId) {
        self.terminal_runs.swap_remove(&run_id);
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
}
