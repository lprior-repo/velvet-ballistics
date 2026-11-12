impl StorageRuntimeJournal {
    fn storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<JournalEvent> {
        // Dispatch on `&event` so the full event is cloned at most once, by the
        // matched arm that actually consumes it. The previous chain cloned the
        // event three times before the real variant was determined.
        let result = match &event {
            RuntimeJournalEvent::RunSubmitted { .. }
            | RuntimeJournalEvent::RunAdmission { .. }
            | RuntimeJournalEvent::CompatibilityDecision { .. }
            | RuntimeJournalEvent::TenantAdmission { .. }
            | RuntimeJournalEvent::RetryCheckpoint { .. }
            | RuntimeJournalEvent::RunFinished { .. }
            | RuntimeJournalEvent::RunFailed { .. }
            | RuntimeJournalEvent::RunCancelled { .. }
            | RuntimeJournalEvent::RunKilled { .. }
            | RuntimeJournalEvent::Resumed { .. }
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. }
            | RuntimeJournalEvent::StepFailed { .. } => {
                Self::run_storage_event(clone_for_dispatch(&event), seq)
            }
            RuntimeJournalEvent::ActionScheduled { .. }
            | RuntimeJournalEvent::ActionCompleted { .. }
            | RuntimeJournalEvent::ActionScheduledTicket { .. }
            | RuntimeJournalEvent::ActionCompletedEnvelope { .. }
            | RuntimeJournalEvent::ActionFailed { .. }
            | RuntimeJournalEvent::ActionAbandoned { .. } => {
                Ok(Self::action_storage_event(clone_for_dispatch(&event), seq))
            }
            RuntimeJournalEvent::RetryTimerScheduled { .. }
            | RuntimeJournalEvent::WaitScheduled { .. }
            | RuntimeJournalEvent::WaitResolved { .. }
            | RuntimeJournalEvent::AskScheduled { .. }
            | RuntimeJournalEvent::AskAnswered { .. }
            | RuntimeJournalEvent::AskTimedOut { .. }
            | RuntimeJournalEvent::SlotWritten { .. } => {
                Self::boundary_storage_event(clone_for_dispatch(&event), seq)
            }
        }?;
        if let Some(storage_event) = result {
            return Ok(storage_event);
        }
        Err(RuntimeError::UnsupportedRuntimeJournalEventMapping {
            event_kind: event.kind(),
        })
    }
}

fn resumed_storage_event(run: RunId, seq: EventSeq, timestamp: u64) -> RuntimeResult<JournalEvent> {
    let seconds =
        i64::try_from(timestamp).map_err(|_| RuntimeError::RuntimeJournalTimestampOutOfRange {
            event_kind: "Resumed",
            timestamp,
        })?;
    let timestamp = DateTime::<Utc>::from_timestamp(seconds, 0).ok_or(
        RuntimeError::RuntimeJournalTimestampOutOfRange {
            event_kind: "Resumed",
            timestamp,
        },
    )?;
    Ok(JournalEvent::RunResumed {
        run,
        seq,
        timestamp,
    })
}

// Counts the number of times `clone_for_dispatch` is invoked on the
// current thread. Used by the
// `storage_event_clones_the_event_exactly_once_per_dispatch` regression test
// to assert that dispatching a `RuntimeJournalEvent` through `storage_event`
// performs exactly one full-event clone. Thread-local so concurrently
// running tests cannot inflate each other's counts; each test resets its
// own thread's count before asserting.
#[cfg(test)]
std::thread_local! {
    static STORAGE_EVENT_CLONE_COUNT: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

/// Clones a `RuntimeJournalEvent` for dispatch into one of the per-variant
/// helpers. In test builds this also increments the calling thread's
/// `STORAGE_EVENT_CLONE_COUNT` so the single-clone invariant is enforced by
/// an executable assertion; in release builds it is a plain clone.
fn clone_for_dispatch(event: &RuntimeJournalEvent) -> RuntimeJournalEvent {
    #[cfg(test)]
    {
        STORAGE_EVENT_CLONE_COUNT.with(|count| count.set(count.get().saturating_add(1)));
    }
    event.clone()
}

fn encoded_slot_taint_extra(
    taint: Taint,
    extra: Option<Vec<u8>>,
) -> RuntimeResult<Option<Vec<u8>>> {
    vb_storage::encode_slot_written_extra(taint, extra)
        .map(Some)
        .map_err(|_| RuntimeError::EncodeFailed)
}

impl RuntimeJournal for StorageRuntimeJournal {
    fn executor_catalog(&self) -> Option<Arc<vb_storage::executor::ExecutorCatalog>> {
        self.catalog.clone()
    }

    fn append(&self, _event: RuntimeJournalEvent) -> RuntimeResult<()> {
        Err(RuntimeError::UnsupportedOperation {
            operation: "unsequenced_storage_journal_append",
        })
    }

    fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
        let storage_event = Self::storage_event(event, seq)?;
        self.append_storage_event(&storage_event)?;
        Ok(())
    }
    fn convert_to_storage_event(
        &self,
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<vb_storage::JournalEvent> {
        Self::storage_event(event, seq)
    }

    fn append_policy_decision_strict(
        &self,
        decision: vb_core::policy::DecisionRecord,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        let storage_event = JournalEvent::PolicyDecisionEvent {
            run: decision.ticket.run,
            seq,
            decision,
        };
        self.journal
            .append_strict(&storage_event)
            .map_err(RuntimeError::from)
    }

    fn append_gate_decision_strict(
        &self,
        record: vb_core::manual_gate::GateOutcomeRecord,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        let storage_event = JournalEvent::GateDecisionEvent {
            run: record.run,
            seq,
            record: Box::new(record),
        };
        self.journal
            .append_strict(&storage_event)
            .map_err(RuntimeError::from)
    }

    fn append_gate_opened_strict(
        &self,
        gate: vb_core::manual_gate::ManualGate,
        opened_at_ms: u64,
        requester: vb_core::manual_gate::ApproverId,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        let storage_event = JournalEvent::GateOpenedEvent {
            run: gate.run,
            seq,
            gate,
            opened_at_ms,
            requester,
        };
        self.journal
            .append_strict(&storage_event)
            .map_err(RuntimeError::from)
    }

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        start_seq: EventSeq,
    ) -> RuntimeResult<()> {
        Self::append_runtime_event_batch(self.journal.as_ref(), self.profile, events, start_seq)
    }

    fn probe(&self) -> RuntimeResult<()> {
        self.journal.probe_health().map_err(RuntimeError::from)
    }

    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        Some(self.profile)
    }

    fn storage_journal(&self) -> Option<std::sync::Arc<vb_storage::FjallJournal>> {
        Some(self.journal.clone())
    }
}

/// Runtime journal adapter that stages lifecycle events through `JournalWriterQueue`.
pub struct QueuedStorageRuntimeJournal {
    journal: Arc<FjallJournal>,
    queue: Arc<JournalWriterQueue>,
    profile: DurabilityProfile,
}

impl QueuedStorageRuntimeJournal {
    /// Creates a queued adapter that enqueues journaled requests.
    #[must_use]
    pub fn journaled(journal: Arc<FjallJournal>, queue: Arc<JournalWriterQueue>) -> Self {
        Self {
            journal,
            queue,
            profile: DurabilityProfile::Journaled,
        }
    }

    /// Creates a queued adapter that enqueues strict requests.
    #[must_use]
    pub fn strict(journal: Arc<FjallJournal>, queue: Arc<JournalWriterQueue>) -> Self {
        Self {
            journal,
            queue,
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
