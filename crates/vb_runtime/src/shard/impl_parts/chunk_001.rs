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
            secret_availability: std::sync::Arc::from([]),
            command_queue: ShardCommandQueue::from_config(config),
            runs: IndexMap::new(),
            run_aggregate: RunAggregate::new(),
            journal_sequences: IndexMap::new(),
            pending_ask_authorities: IndexMap::new(),
            open_gates: IndexMap::new(),
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
            completion_capture: crate::runtime::completion_observation::CompletionCapture::default(
            ),
            fencing_tokens: super::fencing::FencingTokens::new(),
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
                std::sync::Arc::new(crate::admission::StorageArtifactStore::with_catalog(
                    fjall_journal,
                    journal.executor_catalog(),
                ))
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
}

/// Owns the exclusive shard borrow so unwind cleanup cannot leave capture enabled.
struct CompletionCaptureGuard<'a> {
    shard: &'a mut Shard,
}

impl CompletionCaptureGuard<'_> {
    fn tick(
        &mut self,
    ) -> (
        RuntimeResult<bool>,
        crate::runtime::completion_observation::CompletionObservationSlot,
    ) {
        let execution = self.shard.tick_inner();
        let captured = core::mem::take(&mut self.shard.completion_capture.slot);
        (execution, captured)
    }
}

impl Drop for CompletionCaptureGuard<'_> {
    fn drop(&mut self) {
        self.shard.completion_capture.reset();
    }
}
