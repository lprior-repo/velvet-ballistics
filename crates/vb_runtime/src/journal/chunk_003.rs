impl RuntimeJournal for QueuedStorageRuntimeJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> RuntimeResult<()> {
        Err(RuntimeError::UnsupportedOperation {
            operation: "unsequenced_storage_journal_append",
        })
    }

    fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
        if self.profile == DurabilityProfile::Strict {
            return Err(RuntimeError::UnsupportedAsyncStrictAck);
        }
        let storage_event = StorageRuntimeJournal::storage_event(event, seq)?;
        let result = self.queue.enqueue_journaled(storage_event);
        result.map_err(RuntimeError::from)?;
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        start_seq: EventSeq,
    ) -> RuntimeResult<()> {
        if self.profile == DurabilityProfile::Strict {
            return Err(RuntimeError::UnsupportedAsyncStrictAck);
        }
        let mut storage_events = Vec::new();
        storage_events
            .try_reserve(events.len())
            .map_err(|_| RuntimeError::from(vb_storage::JournalError::QueueFull))?;
        for (offset, event) in events.iter().enumerate() {
            let seq = StorageRuntimeJournal::sequence_at_offset(start_seq, offset)?;
            let storage_event = StorageRuntimeJournal::storage_event(event.clone(), seq)?;
            storage_events.push(storage_event);
        }
        self.queue
            .enqueue_journaled_batch(storage_events)
            .map_err(RuntimeError::from)
    }

    fn probe(&self) -> RuntimeResult<()> {
        if self.profile == DurabilityProfile::Strict {
            return Err(RuntimeError::UnsupportedAsyncStrictAck);
        }
        self.journal.probe_health().map_err(RuntimeError::from)?;
        self.queue
            .probe_accepting_writes()
            .map_err(RuntimeError::from)
    }

    fn drain_for_shutdown(&self) -> RuntimeResult<JournalWriterFlushReport> {
        // Retire each queue batch only after its own SyncAll succeeds. A
        // failed barrier keeps that batch in RAM for an idempotent retry.
        let report = self.queue.shutdown_durable(&self.journal)?;
        // Also covers writes previously flushed through the profile-driven API.
        self.journal.persist_strict()?;
        Ok(report)
    }

    fn storage_journal(&self) -> Option<std::sync::Arc<vb_storage::FjallJournal>> {
        Some(self.journal.clone())
    }
    fn convert_to_storage_event(
        &self,
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<vb_storage::JournalEvent> {
        StorageRuntimeJournal::storage_event(event, seq)
    }

    fn append_policy_decision_strict(
        &self,
        decision: vb_core::policy::DecisionRecord,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.queue
            .probe_accepting_writes()
            .map_err(RuntimeError::from)?;
        self.queue
            .drain_all(self.journal.as_ref())
            .map_err(RuntimeError::from)?;
        let storage_event = JournalEvent::PolicyDecisionEvent {
            run: decision.ticket.run,
            seq,
            decision,
        };
        self.journal
            .append_strict(&storage_event)
            .map_err(RuntimeError::from)
    }
}
