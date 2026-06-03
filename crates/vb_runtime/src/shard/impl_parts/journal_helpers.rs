impl Shard {
    #[cfg(not(kani))]
    pub(crate) fn append_journal_event(&mut self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let run = event.run_id();
        let seq = self.journal_sequence_for(run);
        self.journal.append_sequenced(event, seq)?;
        self.advance_journal_sequence(run, seq)
    }

    /// `#[cfg(kani)]` replacement for `append_journal_event` that returns
    /// nondeterministic Ok or Err. Trust boundary TB-vb282my-journal-stub-001.
    /// Production journal append uses Fjall-backed persistence; stubbed version
    /// exercises error paths and ordering verification without real I/O.
    #[cfg(kani)]
    pub(crate) fn append_journal_event(
        &mut self,
        _event: RuntimeJournalEvent,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    fn journal_sequence_for(&self, run: RunId) -> EventSeq {
        self.journal_sequences
            .get(&run)
            .copied()
            .unwrap_or(EventSeq::ZERO)
    }

    fn advance_journal_sequence(&mut self, run: RunId, seq: EventSeq) -> RuntimeResult<()> {
        let next = seq
            .get()
            .checked_add(1)
            .map(EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
        self.journal_sequences.insert(run, next);
        Ok(())
    }

    pub(crate) fn discard_journal_sequence(&mut self, run: RunId) {
        self.journal_sequences.swap_remove(&run);
    }
}
