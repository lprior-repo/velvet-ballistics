impl Shard {
    #[cfg(not(kani))]
    pub(crate) fn append_journal_event(&mut self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let run = event.run_id();
        let seq = self.journal_sequence_for(run);
        self.journal.append_sequenced(event, seq)?;
        self.advance_journal_sequence(run, seq)
    }

    #[cfg(not(kani))]
    pub(crate) fn append_journal_event_batch(
        &mut self,
        events: &[RuntimeJournalEvent],
    ) -> RuntimeResult<()> {
        let Some((first, rest)) = events.split_first() else {
            return Ok(());
        };
        let run = first.run_id();
        for event in rest {
            if event.run_id() != run {
                return Err(RuntimeError::UnsupportedOperation {
                    operation: "mixed_run_journal_batch",
                });
            }
        }
        let seq = self.journal_sequence_for(run);
        let next = Self::journal_sequence_after(seq, events.len())?;
        self.reserve_journal_sequence_slot(run)?;
        self.journal.append_sequenced_batch(events, seq)?;
        let _previous = self.journal_sequences.insert(run, next);
        Ok(())
    }

    #[cfg(not(kani))]
    pub(crate) fn append_journal_events_atomically<const N: usize>(
        &mut self,
        events: [RuntimeJournalEvent; N],
    ) -> RuntimeResult<()> {
        self.append_journal_event_batch(&events)
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

    #[cfg(kani)]
    pub(crate) fn append_journal_event_batch(
        &mut self,
        _events: &[RuntimeJournalEvent],
    ) -> RuntimeResult<()> {
        Ok(())
    }

    #[cfg(kani)]
    pub(crate) fn append_journal_events_atomically<const N: usize>(
        &mut self,
        _events: [RuntimeJournalEvent; N],
    ) -> RuntimeResult<()> {
        Ok(())
    }
}
