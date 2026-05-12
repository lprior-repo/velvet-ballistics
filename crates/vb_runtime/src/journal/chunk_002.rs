impl StorageRuntimeJournal {
    /// Creates an adapter that uses journaled appends without a per-event sync barrier.
    #[must_use]
    pub fn journaled(journal: Arc<FjallJournal>) -> Self {
        Self {
            journal,
            next_seq_by_run: Mutex::new(IndexMap::new()),
            profile: DurabilityProfile::Journaled,
        }
    }

    /// Creates an adapter that forces a strict durability barrier for every append.
    #[must_use]
    pub fn strict(journal: Arc<FjallJournal>) -> Self {
        Self {
            journal,
            next_seq_by_run: Mutex::new(IndexMap::new()),
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

    fn run_storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
        match event {
            RuntimeJournalEvent::RunSubmitted { run, workflow } => {
                Some(JournalEvent::RunAccepted { run, seq, workflow })
            }
            RuntimeJournalEvent::RunAdmission { admission } => Some(JournalEvent::RunAdmission {
                run: admission.run_id(),
                seq,
                artifact_digest: admission.artifact_digest(),
                granted_capabilities: admission.granted_capabilities().clone(),
                policy: admission.policy(),
            }),
            RuntimeJournalEvent::RunFinished { run, result } => {
                Some(JournalEvent::RunFinished { run, seq, result, attempt: 1 })
            }
            RuntimeJournalEvent::RunFailed { run } => {
                Some(JournalEvent::RunFailedEvent { run, seq, attempt: 1 })
            }
            RuntimeJournalEvent::RunCancelled { run, reason } => {
                Some(JournalEvent::RunCancelled { run, seq, attempt: 1, reason })
            }
            RuntimeJournalEvent::StepStarted { run, step } => {
                Some(JournalEvent::StepStarted { run, seq, step, attempt: 1 })
            }
            RuntimeJournalEvent::StepSucceeded { run, step, output, .. } => {
                Some(JournalEvent::StepSucceeded {
                    run,
                    seq,
                    step,
                    output,
                })
            }
            RuntimeJournalEvent::ActionScheduled { .. }
            | RuntimeJournalEvent::ActionCompleted { .. }
            | RuntimeJournalEvent::ActionFailed { .. }
            | RuntimeJournalEvent::WaitScheduled { .. }
            | RuntimeJournalEvent::WaitResolved { .. }
            | RuntimeJournalEvent::AskScheduled { .. }
            | RuntimeJournalEvent::AskAnswered { .. }
            | RuntimeJournalEvent::SlotWritten { .. }
            | RuntimeJournalEvent::Resumed { .. } => None,
        }
    }

    fn action_storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
        match event {
            RuntimeJournalEvent::ActionScheduled { run, step, action } => {
                Some(JournalEvent::ActionScheduled {
                    run,
                    seq,
                    step,
                    action,
                    attempt: 1,
                })
            }
            RuntimeJournalEvent::ActionCompleted { run, step, action } => {
                Some(JournalEvent::ActionCompletedEvent {
                    run,
                    seq,
                    step,
                    action,
                    attempt: 1,
                })
            }
            RuntimeJournalEvent::ActionFailed { run, step, action, attempt } => {
                Some(JournalEvent::ActionFailedEvent {
                    run,
                    seq,
                    step,
                    action,
                    attempt,
                })
            }
            RuntimeJournalEvent::RunSubmitted { .. }
            | RuntimeJournalEvent::RunAdmission { .. }
            | RuntimeJournalEvent::RunFinished { .. }
            | RuntimeJournalEvent::RunFailed { .. }
            | RuntimeJournalEvent::RunCancelled { .. }
            | RuntimeJournalEvent::WaitScheduled { .. }
            | RuntimeJournalEvent::WaitResolved { .. }
            | RuntimeJournalEvent::AskScheduled { .. }
            | RuntimeJournalEvent::AskAnswered { .. }
            | RuntimeJournalEvent::SlotWritten { .. }
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. }
            | RuntimeJournalEvent::Resumed { .. } => None,
        }
    }

    fn boundary_storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
        match event {
            RuntimeJournalEvent::WaitScheduled { run, step } => {
                Some(JournalEvent::WaitScheduledEvent { run, seq, step, attempt: 1 })
            }
            RuntimeJournalEvent::WaitResolved { run, step } => {
                Some(JournalEvent::RetryScheduledEvent { run, seq, step, attempt: 1 })
            }
            RuntimeJournalEvent::AskScheduled { run, step } => {
                Some(JournalEvent::AskScheduledEvent { run, seq, step, attempt: 1 })
            }
            RuntimeJournalEvent::AskAnswered { run, step, .. } => {
                Some(JournalEvent::AskAnsweredEvent { run, seq, step, attempt: 1 })
            }
            RuntimeJournalEvent::SlotWritten {
                run,
                slot,
                value,
                taint,
                extra,
            } => Some(JournalEvent::SlotWrittenEvent {
                run,
                seq,
                slot,
                value: Some(value),
                extra: encoded_slot_taint_extra(taint, extra),
                attempt: 1,
            }),
            RuntimeJournalEvent::RunSubmitted { .. }
            | RuntimeJournalEvent::RunAdmission { .. }
            | RuntimeJournalEvent::RunFinished { .. }
            | RuntimeJournalEvent::RunFailed { .. }
            | RuntimeJournalEvent::RunCancelled { .. }
            | RuntimeJournalEvent::ActionScheduled { .. }
            | RuntimeJournalEvent::ActionCompleted { .. }
            | RuntimeJournalEvent::ActionFailed { .. }
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. }
            | RuntimeJournalEvent::Resumed { .. } => None,
        }
    }

    fn storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> JournalEvent {
        if let Some(storage_event) = Self::run_storage_event(event.clone(), seq) {
            return storage_event;
        }
        if let Some(storage_event) = Self::action_storage_event(event.clone(), seq) {
            return storage_event;
        }
        match Self::boundary_storage_event(event.clone(), seq) {
            Some(storage_event) => storage_event,
            None => JournalEvent::RunFailedEvent {
                run: event.run_id(),
                seq,
                attempt: 1,
            },
        }
    }
}

fn encoded_slot_taint_extra(taint: Taint, extra: Option<Vec<u8>>) -> Option<Vec<u8>> {
    extra.or_else(|| postcard::to_allocvec(&taint).ok())
}

impl RuntimeJournal for StorageRuntimeJournal {
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let run_id = event.run_id();
        let mut sequences = self
            .next_seq_by_run
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        let seq = current_seq(&sequences, run_id);
        let next = next_seq(seq)?;
        let storage_event = Self::storage_event(event, seq);
        self.append_storage_event(&storage_event)?;
        sequences.insert(run_id, next);
        Ok(())
    }
}

/// Runtime journal adapter that stages lifecycle events through `JournalWriterQueue`.
pub struct QueuedStorageRuntimeJournal {
    journal: Arc<FjallJournal>,
    queue: Arc<JournalWriterQueue>,
    next_seq_by_run: Mutex<IndexMap<RunId, EventSeq>>,
    profile: DurabilityProfile,
}

impl QueuedStorageRuntimeJournal {
    /// Creates a queued adapter that enqueues journaled requests.
    #[must_use]
    pub fn journaled(journal: Arc<FjallJournal>, queue: Arc<JournalWriterQueue>) -> Self {
        Self {
            journal,
            queue,
            next_seq_by_run: Mutex::new(IndexMap::new()),
            profile: DurabilityProfile::Journaled,
        }
    }

    /// Creates a queued adapter that enqueues strict requests.
    #[must_use]
    pub fn strict(journal: Arc<FjallJournal>, queue: Arc<JournalWriterQueue>) -> Self {
        Self {
            journal,
            queue,
            next_seq_by_run: Mutex::new(IndexMap::new()),
            profile: DurabilityProfile::Strict,
        }
    }

    /// Creates a shared queued journaled adapter for direct runtime construction.
    #[must_use]
    pub fn shared_journaled(
        journal: Arc<FjallJournal>,
        queue: Arc<JournalWriterQueue>,
    ) -> SharedRuntimeJournal {
        Arc::new(Self::journaled(journal, queue))
    }

    /// Creates a shared queued strict adapter for direct runtime construction.
    #[must_use]
    pub fn shared_strict(
        journal: Arc<FjallJournal>,
        queue: Arc<JournalWriterQueue>,
    ) -> SharedRuntimeJournal {
        Arc::new(Self::strict(journal, queue))
    }

    /// Flushes a bounded batch from the queue into Fjall.
    pub fn flush_batch(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.queue
            .flush_batch(&self.journal)
            .map_err(RuntimeError::from)
    }

    /// Drains all queued journal writes into Fjall.
    pub fn drain_all(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.queue
            .drain_all(&self.journal)
            .map_err(RuntimeError::from)
    }
}
