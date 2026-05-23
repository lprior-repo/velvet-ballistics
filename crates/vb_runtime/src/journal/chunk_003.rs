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

    fn probe(&self) -> RuntimeResult<()> {
        Ok(())
    }

    fn drain_for_shutdown(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.drain_all()
    }

    fn storage_journal(&self) -> Option<std::sync::Arc<vb_storage::FjallJournal>> {
        Some(self.journal.clone())
    }
}
