/// Builds the tenant admission receipt for a tenant-scoped admission.
///
/// Returns `None` when the admission carries no tenant, or when it carries
/// a tenant but no admitted budget to charge: the receipt records admitted
/// cost units, and a tenant without a budget has no cost to record.
#[must_use]
pub(crate) fn tenant_admission_event(
    admission: &crate::admission::RunAdmission,
    run: RunId,
) -> Option<RuntimeJournalEvent> {
    let tenant = admission.tenant()?;
    let budget = admission.budget()?;
    Some(RuntimeJournalEvent::TenantAdmission {
        run,
        tenant,
        cost_units: budget.max_cost_units,
    })
}

impl RuntimeJournalEvent {
    /// Run identifier carried by this runtime event.
    #[must_use]
    pub fn run_id(&self) -> RunId {
        match self {
            Self::RunSubmitted { run, .. }
            | Self::RunFinished { run, .. }
            | Self::RunFailed { run, .. }
            | Self::RunCancelled { run, .. }
            | Self::RunKilled { run, .. }
            | Self::ActionScheduled { run, .. }
            | Self::ActionCompleted { run, .. }
            | Self::ActionFailed { run, .. }
            | Self::WaitScheduled { run, .. }
            | Self::WaitResolved { run, .. }
            | Self::AskScheduled { run, .. }
            | Self::AskAnswered { run, .. }
            | Self::AskTimedOut { run, .. }
            | Self::SlotWritten { run, .. }
            | Self::StepStarted { run, .. }
            | Self::StepSucceeded { run, .. }
            | Self::Resumed { run, .. } => *run,
            Self::ActionScheduledTicket { ticket, .. }
            | Self::ActionCompletedEnvelope { ticket, .. }
            | Self::ActionAbandoned { ticket } => ticket.run,
            Self::RunAdmission { admission } => admission.run_id(),
            Self::CompatibilityDecision { run, .. }
            | Self::TenantAdmission { run, .. } => *run,
            Self::RetryCheckpoint { run, .. } => *run,
        }
    }

    /// Stable variant name for typed storage-mapping diagnostics.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::RunSubmitted { .. } => "RunSubmitted",
            Self::RunAdmission { .. } => "RunAdmission",
            Self::CompatibilityDecision { .. } => "CompatibilityDecision",
            Self::TenantAdmission { .. } => "TenantAdmission",
            Self::RetryCheckpoint { .. } => "RetryCheckpoint",
            Self::RunFinished { .. } => "RunFinished",
            Self::RunFailed { .. } => "RunFailed",
            Self::RunCancelled { .. } => "RunCancelled",
            Self::RunKilled { .. } => "RunKilled",
            Self::ActionScheduled { .. } => "ActionScheduled",
            Self::ActionCompleted { .. } => "ActionCompleted",
            Self::ActionScheduledTicket { .. } => "ActionScheduledTicket",
            Self::ActionCompletedEnvelope { .. } => "ActionCompletedEnvelope",
            Self::ActionFailed { .. } => "ActionFailed",
            Self::ActionAbandoned { .. } => "ActionAbandoned",
            _ => self.kind_tail(),
        }
    }

    /// Variant names for the wait/ask/slot/step/resume tail of `kind`.
    /// Only reachable from `kind` for those variants, so the fallback is
    /// unreachable by construction; it yields an empty name rather than
    /// attributing the event to the wrong variant.
    const fn kind_tail(&self) -> &'static str {
        match self {
            Self::WaitScheduled { .. } => "WaitScheduled",
            Self::WaitResolved { .. } => "WaitResolved",
            Self::AskScheduled { .. } => "AskScheduled",
            Self::AskAnswered { .. } => "AskAnswered",
            Self::AskTimedOut { .. } => "AskTimedOut",
            Self::SlotWritten { .. } => "SlotWritten",
            Self::StepStarted { .. } => "StepStarted",
            Self::StepSucceeded { .. } => "StepSucceeded",
            Self::Resumed { .. } => "Resumed",
            _ => "",
        }
    }
}

/// Append-only port used by runtime shards for lifecycle journaling.
pub trait RuntimeJournal: Send + Sync {
    /// Actual append durability, when explicitly exposed by the adapter.
    /// Unknown/default adapters carry no strict activation authority.
    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        None
    }

    /// Explicit trusted startup catalog; raw serialized reports cannot create this value.
    fn executor_catalog(&self) -> Option<Arc<vb_storage::executor::ExecutorCatalog>> {
        None
    }

    /// Appends a lifecycle event.
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()>;

    /// Returns the underlying Fjall journal if this is a storage-backed journal.
    ///
    /// Returns `Some(Arc<FjallJournal>)` for `StorageRuntimeJournal`, or `None` for
    /// noop/volatile journals. This allows callers to construct a
    /// `StorageArtifactStore` from a storage-backed journal for strict/journaled
    /// artifact admission.
    fn storage_journal(&self) -> Option<std::sync::Arc<vb_storage::FjallJournal>> {
        None
    }

    /// Appends a lifecycle event whose per-run sequence is owned by the shard.
    fn append_sequenced(&self, event: RuntimeJournalEvent, _seq: EventSeq) -> RuntimeResult<()> {
        self.append(event)
    }
    /// Converts a `RuntimeJournalEvent` into a storage `JournalEvent` with the
    /// given sequence. Only `StorageRuntimeJournal` and `QueuedStorageRuntimeJournal`
    /// can perform this conversion; noop/volatile journals return
    /// `UnsupportedOperation` since they lack a storage event format.
    fn convert_to_storage_event(
        &self,
        _event: RuntimeJournalEvent,
        _seq: EventSeq,
    ) -> RuntimeResult<vb_storage::JournalEvent> {
        Err(RuntimeError::UnsupportedOperation {
            operation: "storage_event_conversion",
        })
    }

    /// Appends a same-run lifecycle batch whose first per-run sequence is owned
    /// by the shard.
    ///
    /// The default fails closed for multi-event batches so callers cannot get a
    /// partial durable prefix from a journal implementation that has not opted
    /// into atomic batch semantics.
    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        start_seq: EventSeq,
    ) -> RuntimeResult<()> {
        let Some((event, rest)) = events.split_first() else {
            return Ok(());
        };
        if !rest.is_empty() {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "runtime_journal_batch_append",
            });
        }
        self.append_sequenced(event.clone(), start_seq)
    }

    /// Appends a policy decision with strict durable acknowledgement at the
    /// given per-run sequence. The default fails closed for implementations
    /// that lack a strict port (noop and volatile journals).
    fn append_policy_decision_strict(
        &self,
        _decision: vb_core::policy::DecisionRecord,
        _seq: EventSeq,
    ) -> RuntimeResult<()> {
        Err(RuntimeError::UnsupportedOperation {
            operation: "append_policy_decision_strict",
        })
    }

    /// Appends a gate decision with strict durable acknowledgement at the
    /// given per-run sequence. The default fails closed for implementations
    /// that lack a strict port (noop and volatile journals).
    fn append_gate_decision_strict(
        &self,
        _record: vb_core::manual_gate::GateOutcomeRecord,
        _seq: EventSeq,
    ) -> RuntimeResult<()> {
        Err(RuntimeError::UnsupportedOperation {
            operation: "append_gate_decision_strict",
        })
    }

    /// Appends a gate opening with strict durable acknowledgement at the
    /// given per-run sequence. The default fails closed for implementations
    /// that lack a strict port (noop and volatile journals).
    fn append_gate_opened_strict(
        &self,
        _gate: vb_core::manual_gate::ManualGate,
        _opened_at_ms: u64,
        _requester: vb_core::manual_gate::ApproverId,
        _seq: EventSeq,
    ) -> RuntimeResult<()> {
        Err(RuntimeError::UnsupportedOperation {
            operation: "append_gate_opened_strict",
        })
    }

    /// Probes journal health without side effects.
    /// Returns `JournalPoisoned` if the underlying storage is unavailable.
    fn probe(&self) -> RuntimeResult<()>;

    /// Drains queued durable writes during graceful shutdown.
    fn drain_for_shutdown(&self) -> RuntimeResult<JournalWriterFlushReport> {
        Ok(JournalWriterFlushReport {
            drained: 0,
            written: 0,
        })
    }

    /// Highest per-run sequence still waiting for durability, or `None` when
    /// every event emitted for the run is durable. Synchronous journals return
    /// `Ok(None)`: `append_*` succeeding means durable.
    fn pending_high_water_for_run(&self, _run: RunId) -> RuntimeResult<Option<EventSeq>> {
        Ok(None)
    }

    /// Makes the run's emitted prefix durable before a retry proceeds.
    /// Synchronous journals are already durable: default no-op.
    fn flush_run_prefix(&self, _run: RunId) -> RuntimeResult<()> {
        Ok(())
    }
}

/// Shared journal trait object.
pub type SharedRuntimeJournal = Arc<dyn RuntimeJournal>;

/// Journal implementation that intentionally drops all events.
#[derive(Debug, Default)]
pub struct NoopRuntimeJournal;

impl NoopRuntimeJournal {
    /// Creates a shared noop journal for explicitly non-durable tests or benchmarks.
    #[must_use]
    pub fn shared_for_tests_and_benchmarks() -> SharedRuntimeJournal {
        Arc::new(Self)
    }

    /// Creates a shared noop journal for callers that explicitly select no durability.
    #[must_use]
    pub fn shared() -> SharedRuntimeJournal {
        Self::shared_for_tests_and_benchmarks()
    }
}

impl RuntimeJournal for NoopRuntimeJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> RuntimeResult<()> {
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        _events: &[RuntimeJournalEvent],
        _start_seq: EventSeq,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    fn probe(&self) -> RuntimeResult<()> {
        Ok(())
    }
}
