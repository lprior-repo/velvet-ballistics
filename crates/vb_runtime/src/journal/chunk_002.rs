// Storage runtime journal adapter using Fjall-backed durability.
//
// - [`StorageRuntimeJournal`](crate::journal::StorageRuntimeJournal): synchronous journal adapter
// - [`QueuedStorageRuntimeJournal`](crate::journal::QueuedStorageRuntimeJournal): queued/batched adapter

mod chunk_002_queued;

pub(crate) use chunk_002_queued::QueuedStorageRuntimeJournal;

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
            RuntimeJournalEvent::RunFinished { run, result } => Some(JournalEvent::RunFinished {
                run,
                seq,
                result,
                attempt: 1,
            }),
            RuntimeJournalEvent::RunFailed { run } => Some(JournalEvent::RunFailedEvent {
                run,
                seq,
                attempt: 1,
            }),
            RuntimeJournalEvent::RunCancelled { run, reason } => Some(JournalEvent::RunCancelled {
                run,
                seq,
                attempt: 1,
                reason,
            }),
            RuntimeJournalEvent::RunKilled { run } => Some(JournalEvent::RunKilled {
                run,
                seq,
                attempt: 1,
            }),
            RuntimeJournalEvent::StepStarted { run, step } => Some(JournalEvent::StepStarted {
                run,
                seq,
                step,
                attempt: 1,
            }),
            RuntimeJournalEvent::StepSucceeded {
                run, step, output, ..
            } => Some(JournalEvent::StepSucceeded {
                run,
                seq,
                step,
                output,
            }),
            RuntimeJournalEvent::ActionScheduled { .. }
            | RuntimeJournalEvent::ActionCompleted { .. }
            | RuntimeJournalEvent::ActionScheduledTicket { .. }
            | RuntimeJournalEvent::ActionCompletedEnvelope { .. }
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
            RuntimeJournalEvent::ActionScheduledTicket {
                ticket,
                input,
                output,
            } => Some(JournalEvent::ActionScheduledTicket {
                run: ticket.run,
                seq,
                ticket,
                input,
                output,
            }),
            RuntimeJournalEvent::ActionCompletedEnvelope {
                ticket,
                output,
                value,
                encoded_len,
                taint,
                value_digest,
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
                Ok(Some(JournalEvent::RetryScheduledEvent {
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
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. }
            | RuntimeJournalEvent::Resumed { .. } => Ok(None),
        }
    }

    fn storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<JournalEvent> {
        if let Some(storage_event) = Self::run_storage_event(event.clone(), seq) {
            return Ok(storage_event);
        }
        if let Some(storage_event) = Self::action_storage_event(event.clone(), seq) {
            return Ok(storage_event);
        }
        match Self::boundary_storage_event(event.clone(), seq)? {
            Some(storage_event) => Ok(storage_event),
            None => Ok(JournalEvent::RunFailedEvent {
                run: event.run_id(),
                seq,
                attempt: 1,
            }),
        }
    }
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
        // Extract action index update before consuming the event
        let action_index_update = if let RuntimeJournalEvent::ActionScheduledTicket {
            ticket,
            ..
        } = &event
        {
            Some((ticket.action, ticket.run, ticket.step))
        } else {
            None
        };

        let storage_event = Self::storage_event(event, seq)?;
        self.append_storage_event(&storage_event)?;

        // Update action index keyspace when scheduling an action
        if let Some((action, run, step)) = action_index_update {
            self.journal.put_action_index(action, run, step)?;
        }

        Ok(())
    }

    fn probe(&self) -> RuntimeResult<()> {
        Ok(())
    }

    fn storage_journal(&self) -> Option<std::sync::Arc<vb_storage::FjallJournal>> {
        Some(self.journal.clone())
    }
}
