impl StorageRuntimeJournal {
    /// Creates an adapter that uses journaled appends without a per-event sync barrier.
    #[must_use]
    pub fn journaled(journal: Arc<FjallJournal>) -> Self {
        Self {
            journal,
            profile: DurabilityProfile::Journaled,
        }
    }

    /// Creates an adapter that forces a strict durability barrier for every append.
    #[must_use]
    pub fn strict(journal: Arc<FjallJournal>) -> Self {
        Self {
            journal,
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
            batch.append_event(&storage_event).map_err(RuntimeError::from)?;
        }
        if profile == DurabilityProfile::Strict {
            return batch.strict().commit().map_err(RuntimeError::from);
        }
        batch.commit().map_err(RuntimeError::from)
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
        match event {
            RuntimeJournalEvent::RunSubmitted { run, workflow } => {
                Ok(Some(JournalEvent::RunAccepted { run, seq, workflow }))
            }
            RuntimeJournalEvent::RunAdmission { admission } => Ok(Some(JournalEvent::RunAdmission {
                run: admission.run_id(),
                seq,
                artifact_digest: admission.artifact_digest(),
                granted_capabilities: admission.granted_capabilities().clone(),
                policy: admission.policy(),
            })),
            RuntimeJournalEvent::RunFinished { run, result } => Ok(Some(JournalEvent::RunFinished {
                run,
                seq,
                result,
                attempt: 1,
            })),
            RuntimeJournalEvent::RunFailed { run } => Ok(Some(JournalEvent::RunFailedEvent {
                run,
                seq,
                attempt: 1,
            })),
            RuntimeJournalEvent::RunCancelled { run, reason } => Ok(Some(JournalEvent::RunCancelled {
                run,
                seq,
                attempt: 1,
                reason,
            })),
            RuntimeJournalEvent::RunKilled { run } => Ok(Some(JournalEvent::RunKilled {
                run,
                seq,
                attempt: 1,
            })),
            RuntimeJournalEvent::Resumed { run, timestamp } => {
                resumed_storage_event(run, seq, timestamp).map(Some)
            }
            RuntimeJournalEvent::StepStarted { run, step } => Ok(Some(JournalEvent::StepStarted {
                run,
                seq,
                step,
                attempt: 1,
            })),
            RuntimeJournalEvent::StepSucceeded {
                run, step, output, ..
            } => Ok(Some(JournalEvent::StepSucceeded {
                run,
                seq,
                step,
                output,
            })),
            RuntimeJournalEvent::ActionScheduled { .. }
            | RuntimeJournalEvent::ActionCompleted { .. }
            | RuntimeJournalEvent::ActionScheduledTicket { .. }
            | RuntimeJournalEvent::ActionCompletedEnvelope { .. }
            | RuntimeJournalEvent::ActionFailed { .. }
            | RuntimeJournalEvent::ActionAbandoned { .. }
            | RuntimeJournalEvent::WaitScheduled { .. }
            | RuntimeJournalEvent::WaitResolved { .. }
            | RuntimeJournalEvent::AskScheduled { .. }
            | RuntimeJournalEvent::AskAnswered { .. }
            | RuntimeJournalEvent::AskTimedOut { .. }
            | RuntimeJournalEvent::SlotWritten { .. } => Ok(None),
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
            RuntimeJournalEvent::ActionScheduledTicket {
                ticket,
                input,
                output,
                action_abi_digest,
            } => Some(JournalEvent::ActionScheduledTicket {
                run: ticket.run,
                seq,
                ticket,
                input,
                output,
                action_abi_digest,
            }),
            RuntimeJournalEvent::ActionCompletedEnvelope {
                ticket,
                output,
                value,
                encoded_len,
                taint,
                value_digest,
                action_abi_digest,
            } => Some(JournalEvent::ActionCompletedEnvelope {
                run: ticket.run,
                seq,
                ticket,
                output,
                outcome: vb_storage::DurableActionOutcome::Ready,
                value,
                encoded_len,
                taint,
                value_digest,
                action_abi_digest,
            }),
            RuntimeJournalEvent::ActionFailed {
                run,
                step,
                action,
                attempt,
            } => Some(JournalEvent::ActionFailedEvent {
                run,
                seq,
                step,
                action,
                attempt,
            }),
            RuntimeJournalEvent::ActionAbandoned { ticket } => Some(JournalEvent::ActionAbandoned {
                run: ticket.run,
                seq,
                ticket,
            }),
            RuntimeJournalEvent::RunSubmitted { .. }
            | RuntimeJournalEvent::RunAdmission { .. }
            | RuntimeJournalEvent::RunFinished { .. }
            | RuntimeJournalEvent::RunFailed { .. }
            | RuntimeJournalEvent::RunCancelled { .. }
            | RuntimeJournalEvent::RunKilled { .. }
            | RuntimeJournalEvent::WaitScheduled { .. }
            | RuntimeJournalEvent::WaitResolved { .. }
            | RuntimeJournalEvent::AskScheduled { .. }
            | RuntimeJournalEvent::AskAnswered { .. }
            | RuntimeJournalEvent::AskTimedOut { .. }
            | RuntimeJournalEvent::SlotWritten { .. }
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. }
            | RuntimeJournalEvent::Resumed { .. } => None,
        }
    }

    fn boundary_storage_event(
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<Option<JournalEvent>> {
        match event {
            RuntimeJournalEvent::WaitScheduled { run, step } => {
                Ok(Some(JournalEvent::WaitScheduledEvent {
                    run,
                    seq,
                    step,
                    attempt: 1,
                }))
            }
            RuntimeJournalEvent::WaitResolved { run, step } => {
                Ok(Some(JournalEvent::WaitResolvedEvent {
                    run,
                    seq,
                    step,
                    attempt: 1,
                }))
            }
            RuntimeJournalEvent::AskScheduled { run, step } => {
                Ok(Some(JournalEvent::AskScheduledEvent {
                    run,
                    seq,
                    step,
                    attempt: 1,
                }))
            }
            RuntimeJournalEvent::AskAnswered { run, step, .. } => {
                Ok(Some(JournalEvent::AskAnsweredEvent {
                    run,
                    seq,
                    step,
                    attempt: 1,
                }))
            }
            RuntimeJournalEvent::AskTimedOut { run, step } => {
                Ok(Some(JournalEvent::AskTimedOutEvent {
                    run,
                    seq,
                    step,
                    attempt: 1,
                }))
            }
            RuntimeJournalEvent::SlotWritten {
                run,
                slot,
                value,
                taint,
                extra,
            } => Ok(Some(JournalEvent::SlotWrittenEvent {
                run,
                seq,
                slot,
                value: Some(value),
                extra: encoded_slot_taint_extra(taint, extra)?,
                attempt: 1,
            })),
            RuntimeJournalEvent::RunSubmitted { .. }
            | RuntimeJournalEvent::RunAdmission { .. }
            | RuntimeJournalEvent::RunFinished { .. }
            | RuntimeJournalEvent::RunFailed { .. }
            | RuntimeJournalEvent::RunCancelled { .. }
            | RuntimeJournalEvent::RunKilled { .. }
            | RuntimeJournalEvent::ActionScheduled { .. }
            | RuntimeJournalEvent::ActionCompleted { .. }
            | RuntimeJournalEvent::ActionScheduledTicket { .. }
            | RuntimeJournalEvent::ActionCompletedEnvelope { .. }
            | RuntimeJournalEvent::ActionFailed { .. }
            | RuntimeJournalEvent::ActionAbandoned { .. }
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. }
            | RuntimeJournalEvent::Resumed { .. } => Ok(None),
        }
    }

    fn storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<JournalEvent> {
        // Dispatch on `&event` so the full event is cloned at most once, by the
        // matched arm that actually consumes it. The previous chain cloned the
        // event three times before the real variant was determined.
        let result = match &event {
            RuntimeJournalEvent::RunSubmitted { .. }
            | RuntimeJournalEvent::RunAdmission { .. }
            | RuntimeJournalEvent::RunFinished { .. }
            | RuntimeJournalEvent::RunFailed { .. }
            | RuntimeJournalEvent::RunCancelled { .. }
            | RuntimeJournalEvent::RunKilled { .. }
            | RuntimeJournalEvent::Resumed { .. }
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. } => {
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
            RuntimeJournalEvent::WaitScheduled { .. }
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
    let seconds = i64::try_from(timestamp).map_err(|_| {
        RuntimeError::RuntimeJournalTimestampOutOfRange {
            event_kind: "Resumed",
            timestamp,
        }
    })?;
    let timestamp = chrono::DateTime::<chrono::Utc>::from_timestamp(seconds, 0).ok_or(
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

/// Counts the number of times `clone_for_dispatch` is invoked. Used by the
/// `storage_event_clones_the_event_exactly_once_per_dispatch` regression test
/// to assert that dispatching a `RuntimeJournalEvent` through `storage_event`
/// performs exactly one full-event clone.
#[cfg(test)]
static STORAGE_EVENT_CLONE_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Clones a `RuntimeJournalEvent` for dispatch into one of the per-variant
/// helpers. In test builds this also increments `STORAGE_EVENT_CLONE_COUNT`
/// so the single-clone invariant is enforced by an executable assertion; in
/// release builds it is a plain clone.
fn clone_for_dispatch(event: &RuntimeJournalEvent) -> RuntimeJournalEvent {
    #[cfg(test)]
    {
        STORAGE_EVENT_CLONE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
