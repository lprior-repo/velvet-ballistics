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
        // RE-021: delegate to a real, non-side-effecting storage health
        // check on the FjallJournal. Additionally, refuse a positive health
        // verdict if the writer queue has been moved into the shutdown state
        // or the queue is full.
        self.journal
            .probe_storage_health()
            .map_err(RuntimeError::from)?;
        if self
            .queue
            .is_shutdown()
            .map_err(|err| RuntimeError::StorageJournalAppend {
                source: std::sync::Arc::new(err),
            })?
        {
            return Err(RuntimeError::StorageJournalAppend {
                source: std::sync::Arc::new(vb_storage::JournalError::QueueShutdown),
            });
        }
        if self.queue.is_full() {
            return Err(RuntimeError::StorageJournalAppend {
                source: std::sync::Arc::new(vb_storage::JournalError::QueueFull),
            });
        }
        Ok(())
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
