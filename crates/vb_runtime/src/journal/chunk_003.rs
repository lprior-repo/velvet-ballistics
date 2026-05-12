impl RuntimeJournal for QueuedStorageRuntimeJournal {
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        if self.profile == DurabilityProfile::Strict {
            return Err(RuntimeError::UnsupportedAsyncStrictAck);
        }
        let run_id = event.run_id();
        let mut sequences = self
            .next_seq_by_run
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        let seq = current_seq(&sequences, run_id);
        let next = next_seq(seq)?;
        let storage_event = StorageRuntimeJournal::storage_event(event, seq);
        let result = self.queue.enqueue_journaled(storage_event);
        result.map_err(RuntimeError::from)?;
        sequences.insert(run_id, next);
        Ok(())
    }

    fn probe(&self) -> RuntimeResult<()> {
        // Verify the mutex is not poisoned.
        let _guard = self
            .next_seq_by_run
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        Ok(())
    }

    fn drain_for_shutdown(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.drain_all()
    }
}

fn current_seq(sequences: &IndexMap<RunId, EventSeq>, run: RunId) -> EventSeq {
    match sequences.get(&run).copied() {
        Some(value) => value,
        None => EventSeq::new(0),
    }
}

fn next_seq(seq: EventSeq) -> RuntimeResult<EventSeq> {
    seq.get()
        .checked_add(1)
        .map(EventSeq::new)
        .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))
}

