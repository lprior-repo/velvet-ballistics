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
            RuntimeJournalEvent::RunAdmission { admission } => {
                if let Some(budget) = admission.budget() {
                    Ok(Some(JournalEvent::RunAdmissionBudget {
                        input_digest: vb_storage::RunInputBinding::LegacyUnbound,
                        run: admission.run_id(),
                        seq,
                        artifact_digest: admission.artifact_digest(),
                        policy_pin: admission.policy_pin(),
                        granted_capabilities: admission.granted_capabilities().clone(),
                        policy: admission.policy(),
                        budget,
                    }))
                } else {
                    Ok(Some(JournalEvent::RunAdmission {
                        input_digest: vb_storage::RunInputBinding::LegacyUnbound,
                        run: admission.run_id(),
                        seq,
                        artifact_digest: admission.artifact_digest(),
                        policy_pin: admission.policy_pin(),
                        granted_capabilities: admission.granted_capabilities().clone(),
                        policy: admission.policy(),
                    }))
                }
            }
            RuntimeJournalEvent::RetryCheckpoint {
                run,
                attempt,
                step,
                payload_digest,
                payload,
            } => Ok(Some(JournalEvent::RetryCheckpoint {
                run,
                seq,
                attempt,
                step,
                payload_digest,
                payload,
            })),
            RuntimeJournalEvent::RunFinished {
                run,
                result,
                attempt,
            } => Ok(Some(JournalEvent::RunFinished {
                run,
                seq,
                result,
                attempt,
            })),
            RuntimeJournalEvent::CompatibilityDecision { run, decision } => {
                Ok(Some(JournalEvent::CompatibilityDecisionEvent {
                    run,
                    seq,
                    decision,
                }))
            }
            RuntimeJournalEvent::TenantAdmission {
                run,
                tenant,
                cost_units,
            } => Ok(Some(JournalEvent::TenantAdmissionEvent {
                run,
                seq,
                tenant,
                cost_units,
            })),
            RuntimeJournalEvent::RunFailed { run, attempt } => {
                Ok(Some(JournalEvent::RunFailedEvent { run, seq, attempt }))
            }
            RuntimeJournalEvent::RunCancelled {
                run,
                reason,
                attempt,
            } => Ok(Some(JournalEvent::RunCancelled {
                run,
                seq,
                attempt,
                reason,
            })),
            RuntimeJournalEvent::RunKilled { run, attempt } => {
                Ok(Some(JournalEvent::RunKilled { run, seq, attempt }))
            }
            RuntimeJournalEvent::Resumed { run, timestamp } => {
                resumed_storage_event(run, seq, timestamp).map(Some)
            }
            RuntimeJournalEvent::StepStarted { run, step, attempt } => {
                Ok(Some(JournalEvent::StepStarted {
                    run,
                    seq,
                    step,
                    attempt,
                }))
            }
            RuntimeJournalEvent::StepFailed { run, step, attempt } => {
                Ok(Some(JournalEvent::StepFailed {
                    run,
                    seq,
                    step,
                    attempt,
                }))
            }
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
            | RuntimeJournalEvent::RetryTimerScheduled { .. }
            | RuntimeJournalEvent::WaitScheduled { .. }
            | RuntimeJournalEvent::WaitResolved { .. }
            | RuntimeJournalEvent::AskScheduled { .. }
            | RuntimeJournalEvent::AskAnswered { .. }
            | RuntimeJournalEvent::AskTimedOut { .. }
            | RuntimeJournalEvent::SlotWritten { .. } => Ok(None),
        }
    }
}
