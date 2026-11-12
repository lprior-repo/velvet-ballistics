impl Shard {
    /// Kani- and test-only accessor for the per-run journal sequence.
    /// Returns the current `EventSeq` recorded for `run_id`, or `None`
    /// if no sequence has been allocated yet. Used by kani harnesses
    /// (e.g. `kani_ask_answer_lifecycle::kani_ask_answer_journal_monotonicity`)
    /// and the timer regressions to inspect monotonicity invariants without
    /// exposing the field outside those builds.
    #[cfg(any(kani, test))]
    pub(crate) fn journal_seq_get(&self, run_id: RunId) -> Option<EventSeq> {
        self.journal_sequences.get(&run_id).copied()
    }

    pub(crate) fn journal_sequence_for(&self, run: RunId) -> EventSeq {
        self.journal_sequences
            .get(&run)
            .copied()
            .unwrap_or(EventSeq::ZERO)
    }

    /// Retry safety gate: every event emitted for the run must be durable
    /// before a retry proceeds. Queued journals flush the run's prefix on
    /// demand; synchronous journals report nothing pending. A still-pending
    /// high-water after the flush blocks with a typed error and mutates
    /// nothing, so the retry stays possible after an external flush.
    pub(crate) fn can_retry(&self, run: RunId) -> RuntimeResult<()> {
        if self.journal.pending_high_water_for_run(run)?.is_none() {
            return Ok(());
        }
        self.journal.flush_run_prefix(run)?;
        if self.journal.pending_high_water_for_run(run)?.is_some() {
            return Err(RuntimeError::RetryBlockedOnUndrainedJournal { run });
        }
        Ok(())
    }

    #[cfg(not(kani))]
    fn journal_sequence_after(seq: EventSeq, count: usize) -> RuntimeResult<EventSeq> {
        let count = u64::try_from(count)
            .map_err(|_| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
        seq.get()
            .checked_add(count)
            .map(EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))
    }

    fn advance_journal_sequence(&mut self, run: RunId, seq: EventSeq) -> RuntimeResult<()> {
        let next = seq
            .get()
            .checked_add(1)
            .map(EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
        self.reserve_journal_sequence_slot(run)?;
        let _previous = self.journal_sequences.insert(run, next);
        Ok(())
    }

    pub(crate) fn discard_journal_sequence(&mut self, run: RunId) {
        let _removed = self.journal_sequences.swap_remove(&run);
    }

    /// Seeds a journal sequence for a run being recovered.
    ///
    /// Recovery provides the next durable sequence from journal replay;
    /// this sets it as the current sequence so future events advance
    /// from the correct position.
    pub(crate) fn seed_journal_sequence(&mut self, run: RunId, seq: EventSeq) -> RuntimeResult<()> {
        self.reserve_journal_sequence_slot(run)?;
        let _previous = self.journal_sequences.insert(run, seq);
        Ok(())
    }
}
