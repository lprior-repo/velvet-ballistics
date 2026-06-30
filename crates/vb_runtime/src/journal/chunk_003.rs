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
        if self.profile == DurabilityProfile::Strict {
            return Err(RuntimeError::UnsupportedAsyncStrictAck);
        }
        self.journal.probe_health().map_err(RuntimeError::from)?;
        self.queue
            .probe_accepting_writes()
            .map_err(RuntimeError::from)
    }

    fn drain_for_shutdown(&self) -> RuntimeResult<JournalWriterFlushReport> {
        // Drain visible writes first, then force a strict durability barrier
        // so every event that left the queue is durable on disk before Ok is
        // returned to the caller (typically `Runtime::shutdown_graceful`).
        // The previous implementation only flushed the queue into Fjall and
        // relied on Fjall's lazy WAL flush; a process crash after
        // `drain_for_shutdown` returned Ok could still lose the just-drained
        // events. `persist_strict` performs `fjall::PersistMode::SyncAll`,
        // satisfying the Master §49 Crash-Consistency Rule.
        let report = self.drain_all()?;
        self.journal.persist_strict()?;
        Ok(report)
    }

    fn storage_journal(&self) -> Option<std::sync::Arc<vb_storage::FjallJournal>> {
        Some(self.journal.clone())
    }
}
