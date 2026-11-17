use crate::shard::types::{RunAggregate, RuntimeState, ShardControlState};

struct ShardInputs {
    quota: crate::quota::SharedQuotaLedger,
    artifact_store: crate::admission::SharedAcceptedArtifactStore,
    journal: SharedRuntimeJournal,
}

struct ShardServices {
    quota: crate::quota::SharedQuotaLedger,
    artifact_store: crate::admission::SharedAcceptedArtifactStore,
    journal: SharedRuntimeJournal,
    queue: ShardCommandQueue,
}

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
        Self::new_with_shared_quota(
            config,
            journal,
            artifact_store,
            crate::quota::SharedQuotaLedger::new(),
        )
    }

    /// Creates a shard sharing a runtime-wide quota ledger.
    pub(crate) fn new_with_shared_quota(
        config: ShardConfig,
        journal: SharedRuntimeJournal,
        artifact_store: crate::admission::SharedAcceptedArtifactStore,
        quota: crate::quota::SharedQuotaLedger,
    ) -> RuntimeResult<Self> {
        let services = ShardInputs {
            quota,
            artifact_store,
            journal,
        };
        Self::try_with_services(config, services)
    }

    fn try_with_services(config: ShardConfig, services: ShardInputs) -> RuntimeResult<Self> {
        validate_command_queue_capacity(config.command_queue_capacity)?;
        let aggregate = RunAggregate::try_with_timer_capacity(config.max_active_runs)?;
        let queue = ShardCommandQueue::from_config(config)?;
        let services = ShardServices {
            quota: services.quota,
            artifact_store: services.artifact_store,
            journal: services.journal,
            queue,
        };
        Ok(Self::with_services(config, services, aggregate))
    }

    fn with_services(config: ShardConfig, ports: ShardServices, aggregate: RunAggregate) -> Self {
        Self {
            secret_availability: std::sync::Arc::from([]),
            command_queue: ports.queue,
            runs: IndexMap::new(),
            run_aggregate: aggregate,
            journal_sequences: IndexMap::new(),
            pending_ask_authorities: IndexMap::new(),
            open_gates: IndexMap::new(),
            bound_open_gates: IndexMap::new(),
            accounted_executed_steps: IndexMap::new(),
            frame_pools: IndexMap::new(),
            trace_ring: TraceRing::new(config.trace_capacity),
            counters: ShardCounters::new(),
            quota: ports.quota,
            pending_quota_reservation: None,
            charged_quotas: IndexMap::new(),
            step_budget_per_tick: config.step_budget_per_tick,
            max_active_runs: config.max_active_runs,
            max_open_gates: config.max_open_gates,
            policy: config.policy,
            artifact_store: ports.artifact_store,
            control: ShardControlState::new(),
            journal: ports.journal,
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
    pub fn new_with_journal(
        config: ShardConfig,
        journal: SharedRuntimeJournal,
    ) -> RuntimeResult<Self> {
        Self::new_with_journal_and_quota(config, journal, crate::quota::SharedQuotaLedger::new())
    }

    /// Creates a shard with a journal sink sharing a runtime-wide quota ledger.
    pub(crate) fn new_with_journal_and_quota(
        config: ShardConfig,
        journal: SharedRuntimeJournal,
        quota: crate::quota::SharedQuotaLedger,
    ) -> RuntimeResult<Self> {
        let artifact_store = Self::artifact_store_for(&config, &journal);
        Self::new_with_shared_quota(config, journal, artifact_store, quota)
    }

    fn artifact_store_for(
        config: &ShardConfig,
        journal: &SharedRuntimeJournal,
    ) -> crate::admission::SharedAcceptedArtifactStore {
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
        }
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
        let captured = core::mem::take(&mut self.shard.control.completion_capture.slot);
        (execution, captured)
    }
}

impl Drop for CompletionCaptureGuard<'_> {
    fn drop(&mut self) {
        self.shard.control.completion_capture.reset();
    }
}
