impl StorageRuntimeJournal {
    /// Strict journal with explicitly checked registered executor startup authority.
    pub fn shared_strict_with_catalog(
        journal: Arc<FjallJournal>,
        catalog: Arc<vb_storage::executor::ExecutorCatalog>,
    ) -> SharedRuntimeJournal {
        Arc::new(Self {
            journal,
            catalog: Some(catalog),
            profile: DurabilityProfile::Strict,
        })
    }

    /// Creates an adapter that uses journaled appends without a per-event sync barrier.
    #[must_use]
    pub fn journaled(journal: Arc<FjallJournal>) -> Self {
        Self {
            journal,
            catalog: None,
            profile: DurabilityProfile::Journaled,
        }
    }

    /// Creates an adapter that forces a strict durability barrier for every append.
    #[must_use]
    pub fn strict(journal: Arc<FjallJournal>) -> Self {
        Self {
            journal,
            catalog: None,
            profile: DurabilityProfile::Strict,
        }
    }

    /// Creates a shared journaled adapter for direct runtime construction.
    #[must_use]
    pub fn shared_journaled(journal: Arc<FjallJournal>) -> SharedRuntimeJournal {
        Arc::new(Self::journaled(journal))
    }

    /// Creates a shared strict adapter for direct runtime construction.
    #[must_use]
    pub fn shared_strict(journal: Arc<FjallJournal>) -> SharedRuntimeJournal {
        Arc::new(Self::strict(journal))
    }

    fn append_storage_event(&self, event: &JournalEvent) -> RuntimeResult<()> {
        let result = if self.profile == DurabilityProfile::Strict {
            self.journal.append_strict(event)
        } else {
            self.journal.append_journaled(event)
        };
        result.map_err(RuntimeError::from)
    }

    fn append_runtime_event_batch(
        journal: &FjallJournal,
        profile: DurabilityProfile,
        events: &[RuntimeJournalEvent],
        start_seq: EventSeq,
    ) -> RuntimeResult<()> {
        if events.is_empty() {
            return Ok(());
        }
        let mut batch = journal.batch();
        for (offset, event) in events.iter().enumerate() {
            let seq = Self::sequence_at_offset(start_seq, offset)?;
            let storage_event = Self::storage_event(event.clone(), seq)?;
            batch
                .append_event(&storage_event)
                .map_err(RuntimeError::from)?;
        }
        if profile == DurabilityProfile::Strict {
            return batch.strict().commit().map_err(RuntimeError::from);
        }
        batch.commit().map_err(RuntimeError::from)
    }

    fn append_runtime_admission_batch(
        journal: &FjallJournal,
        profile: DurabilityProfile,
        events: &[RuntimeJournalEvent],
        start_seq: EventSeq,
        inputs: vb_storage::RunInputsRecord,
    ) -> RuntimeResult<()> {
        let mut storage_events = Vec::new();
        storage_events
            .try_reserve(events.len())
            .map_err(|_| RuntimeError::from(vb_storage::JournalError::QueueFull))?;
        for (offset, event) in events.iter().enumerate() {
            let seq = Self::sequence_at_offset(start_seq, offset)?;
            storage_events.push(Self::storage_event(event.clone(), seq)?);
        }
        journal
            .append_runtime_admission_batch(&storage_events, &inputs, profile)
            .map_err(RuntimeError::from)
    }

    fn sequence_at_offset(start_seq: EventSeq, offset: usize) -> RuntimeResult<EventSeq> {
        let offset = u64::try_from(offset)
            .map_err(|_| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
        start_seq
            .get()
            .checked_add(offset)
            .map(EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))
    }

    fn run_storage_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        use RuntimeJournalEvent::*;
        match event {
            RunAdmission { admission } => Ok(Some(Self::admission_storage_event(admission, seq))),
            event
            @ (RunSubmitted { .. } | RunFailed { .. } | RunKilled { .. } | Resumed { .. }) => {
                Self::run_status_event(event, seq)
            }
            event @ RunFinished { .. } => Self::run_finished_event(event, seq),
            event @ RunCancelled { .. } => Self::run_cancelled_event(event, seq),
            event @ RetryCheckpoint { .. } => Self::run_retry_event(event, seq),
            event @ (CompatibilityDecision { .. } | RunBudgetOrigin { .. }) => {
                Self::run_compatibility_event(event, seq)
            }
            event @ TenantAdmission { .. } => Self::run_tenant_event(event, seq),
            event @ (StepStarted { .. } | StepFailed { .. }) => {
                Self::run_step_status_event(event, seq)
            }
            event @ StepSucceeded { .. } => Self::run_step_success_event(event, seq),
            _ => Ok(None),
        }
    }
}
