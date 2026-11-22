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
        use RuntimeJournalEvent::*;
        match self {
            RunSubmitted { run, .. } | RunBudgetOrigin { run, .. } => *run,
            RunFinished { run, .. } | RunFailed { run, .. } => *run,
            RunCancelled { run, .. } | RunKilled { run, .. } => *run,
            CompatibilityDecision { run, .. } | TenantAdmission { run, .. } => *run,
            RetryCheckpoint { run, .. } | RetryTimerScheduled { run, .. } => *run,
            StepStarted { run, .. } | StepSucceeded { run, .. } | StepFailed { run, .. } => *run,
            ActionScheduled { run, .. }
            | ActionCompleted { run, .. }
            | ActionFailed { run, .. } => *run,
            ActionScheduledTicket { ticket, .. }
            | ActionCompletedEnvelope { ticket, .. }
            | ActionAbandoned { ticket } => ticket.run,
            WaitScheduled { run, .. } | WaitResolved { run, .. } => *run,
            AskScheduled { run, .. } | AskAnswered { run, .. } | AskTimedOut { run, .. } => *run,
            SlotWritten { run, .. } => *run,
            Resumed { run, .. } => *run,
            RunAdmission { admission } => admission.run_id(),
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
            Self::RunBudgetOrigin { .. } => "RunBudgetOrigin",
            Self::RetryCheckpoint { .. } => "RetryCheckpoint",
            Self::RetryTimerScheduled { .. } => "RetryTimerScheduled",
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
            Self::StepFailed { .. } => "StepFailed",
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

    /// Appends same-run batches at the shard-owned sequence; multi-event defaults fail closed to
    /// avoid partial durable prefixes.
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

    /// Atomically appends admission events with their retained scalar inputs.
    /// Durable adapters must implement this operation. The default supports
    /// only explicitly volatile journals, which make no recovery promise.
    fn append_admission_batch(
        &self,
        events: &[RuntimeJournalEvent],
        start_seq: EventSeq,
        _inputs: vb_storage::RunInputsRecord,
    ) -> RuntimeResult<()> {
        if self.durability_profile() != Some(vb_storage::DurabilityProfile::Volatile)
            || self.storage_journal().is_some()
        {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "runtime_journal_admission_batch",
            });
        }
        self.append_sequenced_batch(events, start_seq)
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

    /// Appends a bound gate decision with strict durable acknowledgement.
    /// Default fails closed for journals without the additive port.
    fn append_gate_decision_bound_strict(
        &self,
        _record: vb_core::manual_gate::BoundGateOutcomeRecord,
        _seq: EventSeq,
    ) -> RuntimeResult<()> {
        Err(RuntimeError::UnsupportedOperation {
            operation: "append_gate_decision_bound_strict",
        })
    }

    /// Appends a bound gate denial and its run-failure event in one strict batch.
    /// Default fails closed for journals without the additive port.
    fn append_gate_decision_bound_failing_run_strict(
        &self,
        _record: vb_core::manual_gate::BoundGateOutcomeRecord,
        _seq: EventSeq,
        _run_failed: RuntimeJournalEvent,
    ) -> RuntimeResult<()> {
        Err(RuntimeError::UnsupportedOperation {
            operation: "append_gate_decision_bound_failing_run_strict",
        })
    }

    /// Appends a bound gate opening with strict durable acknowledgement.
    /// Default fails closed for journals without the additive port.
    fn append_gate_opened_bound_strict(
        &self,
        _gate: vb_core::manual_gate::BoundManualGate,
        _opened_at_ms: u64,
        _requester: vb_core::manual_gate::ApproverId,
        _seq: EventSeq,
    ) -> RuntimeResult<()> {
        Err(RuntimeError::UnsupportedOperation {
            operation: "append_gate_opened_bound_strict",
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
