impl Shard {
    /// Strict durable gate decision port.
    ///
    /// Validates the record structurally, computes checked next sequence,
    /// reserves map capacity, measures the stored decision event and
    /// charges it to the run's cumulative journal-bytes counter before
    /// appending, calls the journal's strict port, and advances the
    /// per-run sequence only after success. On any failure the record,
    /// the pending state, the next sequence, and the counter are
    /// unchanged.
    ///
    /// No operator-facing submission flow calls this port yet; the
    /// gate-submission batch wires it up. Until then the port is
    /// exercised by its tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn append_gate_decision_strict(
        &mut self,
        record: vb_core::manual_gate::GateOutcomeRecord,
    ) -> crate::RuntimeResult<()> {
        let run = record.run;

        if !record.is_structurally_valid() {
            return Err(crate::RuntimeError::InvalidGateDecisionRecord);
        }

        let seq = self.journal_sequence_for(run);
        let next = Self::checked_next_seq(seq)?;

        self.reserve_journal_sequence_slot(run)?;

        let event_bytes = Self::gate_decision_event_bytes(&record, seq)?;
        let projected = self
            .run_state_get(run)
            .map(|state| state.journal_bytes_written)
            .ok_or(crate::RuntimeError::RunNotFound)?
            .checked_add(event_bytes)
            .ok_or(crate::RuntimeError::EncodeFailed)?;

        self.journal.append_gate_decision_strict(record, seq)?;

        let _previous = self.journal_sequences.insert(run, next);

        let state = self
            .run_state_get_mut(run)
            .ok_or(crate::RuntimeError::RunNotFound)?;
        state.journal_bytes_written = projected;

        Ok(())
    }

    /// Postcard bytes of the stored gate-decision event.
    ///
    /// The record is stored as the whole storage `JournalEvent`, so the
    /// measure covers the stored record including its run and sequence.
    #[cfg_attr(not(test), allow(dead_code))]
    fn gate_decision_event_bytes(
        record: &vb_core::manual_gate::GateOutcomeRecord,
        seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<u64> {
        let event = vb_storage::JournalEvent::GateDecisionEvent {
            run: record.run,
            seq,
            record: Box::new(record.clone()),
        };
        let encoded =
            postcard::to_allocvec(&event).map_err(|_| crate::RuntimeError::EncodeFailed)?;
        u64::try_from(encoded.len()).map_err(|_| crate::RuntimeError::EncodeFailed)
    }
}
