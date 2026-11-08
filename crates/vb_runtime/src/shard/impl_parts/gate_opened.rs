impl Shard {
    /// Strict durable gate opening port.
    ///
    /// Validates the gate structurally and the requester identity,
    /// computes checked next sequence, reserves map capacity, measures
    /// the stored opened event and charges it to the run's cumulative
    /// journal-bytes counter before appending, calls the journal's
    /// strict port, and advances the per-run sequence only after
    /// success. On any failure the gate, the pending state, the next
    /// sequence, and the counter are unchanged.
    pub(crate) fn append_gate_opened_strict(
        &mut self,
        gate: vb_core::manual_gate::ManualGate,
        opened_at_ms: u64,
        requester: vb_core::manual_gate::ApproverId,
    ) -> crate::RuntimeResult<()> {
        let run = gate.run;

        if !gate.is_structurally_valid() || !requester.is_valid() {
            return Err(crate::RuntimeError::InvalidGateOpenedRecord);
        }

        let seq = self.journal_sequence_for(run);
        let next = Self::checked_next_seq(seq)?;

        self.reserve_journal_sequence_slot(run)?;

        let event_bytes =
            Self::gate_opened_event_bytes(&gate, opened_at_ms, &requester, seq)?;
        let projected = self
            .run_state_get(run)
            .map(|state| state.journal_bytes_written)
            .ok_or(crate::RuntimeError::RunNotFound)?
            .checked_add(event_bytes)
            .ok_or(crate::RuntimeError::EncodeFailed)?;

        self.journal
            .append_gate_opened_strict(gate, opened_at_ms, requester, seq)?;

        let _previous = self.journal_sequences.insert(run, next);

        let state = self
            .run_state_get_mut(run)
            .ok_or(crate::RuntimeError::RunNotFound)?;
        state.journal_bytes_written = projected;

        Ok(())
    }

    /// Postcard bytes of the stored gate-opened event.
    ///
    /// The gate is stored as the whole storage `JournalEvent`, so the
    /// measure covers the stored record including its run and sequence.
    fn gate_opened_event_bytes(
        gate: &vb_core::manual_gate::ManualGate,
        opened_at_ms: u64,
        requester: &vb_core::manual_gate::ApproverId,
        seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<u64> {
        let event = vb_storage::JournalEvent::GateOpenedEvent {
            run: gate.run,
            seq,
            gate: gate.clone(),
            opened_at_ms,
            requester: requester.clone(),
        };
        let encoded =
            postcard::to_allocvec(&event).map_err(|_| crate::RuntimeError::EncodeFailed)?;
        u64::try_from(encoded.len()).map_err(|_| crate::RuntimeError::EncodeFailed)
    }
}
