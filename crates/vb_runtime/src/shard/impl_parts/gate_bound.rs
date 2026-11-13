// Strict durable ports for the additive bound gate events (RecordKind 54/55).

impl Shard {
    /// Strict durable bound gate-opening port. Mirrors the legacy port with
    /// the additive `BoundManualGate` payload and event kind.
    pub(crate) fn append_gate_opened_bound_strict(
        &mut self,
        gate: vb_core::manual_gate::BoundManualGate,
        opened_at_ms: u64,
        requester: vb_core::manual_gate::ApproverId,
    ) -> crate::RuntimeResult<()> {
        let run = gate.gate.run;
        if !gate.is_structurally_valid() || !requester.is_valid() {
            return Err(crate::RuntimeError::InvalidGateOpenedRecord);
        }
        let seq = self.journal_sequence_for(run);
        let next = Self::checked_next_seq(seq)?;
        self.reserve_journal_sequence_slot(run)?;
        let event_bytes =
            Self::gate_opened_bound_event_bytes(&gate, opened_at_ms, &requester, seq)?;
        let projected = self
            .run_state_get(run)
            .map(|state| state.journal_bytes_written)
            .ok_or(crate::RuntimeError::RunNotFound)?
            .checked_add(event_bytes)
            .ok_or(crate::RuntimeError::EncodeFailed)?;
        self.journal
            .append_gate_opened_bound_strict(gate, opened_at_ms, requester, seq)?;
        let _previous = self.journal_sequences.insert(run, next);
        let state = self
            .run_state_get_mut(run)
            .ok_or(crate::RuntimeError::RunNotFound)?;
        state.journal_bytes_written = projected;
        Ok(())
    }

    /// Strict durable bound gate-decision port.
    pub(crate) fn append_gate_decision_bound_strict(
        &mut self,
        record: vb_core::manual_gate::BoundGateOutcomeRecord,
    ) -> crate::RuntimeResult<()> {
        let run = record.record.run;
        if !record.is_structurally_valid() {
            return Err(crate::RuntimeError::InvalidGateDecisionRecord);
        }
        let seq = self.journal_sequence_for(run);
        let next = Self::checked_next_seq(seq)?;
        self.reserve_journal_sequence_slot(run)?;
        let event_bytes = Self::gate_decision_bound_event_bytes(&record, seq)?;
        let projected = self
            .run_state_get(run)
            .map(|state| state.journal_bytes_written)
            .ok_or(crate::RuntimeError::RunNotFound)?
            .checked_add(event_bytes)
            .ok_or(crate::RuntimeError::EncodeFailed)?;
        self.journal
            .append_gate_decision_bound_strict(record, seq)?;
        let _previous = self.journal_sequences.insert(run, next);
        let state = self
            .run_state_get_mut(run)
            .ok_or(crate::RuntimeError::RunNotFound)?;
        state.journal_bytes_written = projected;
        Ok(())
    }

    fn gate_opened_bound_event_bytes(
        gate: &vb_core::manual_gate::BoundManualGate,
        opened_at_ms: u64,
        requester: &vb_core::manual_gate::ApproverId,
        seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<u64> {
        let event = vb_storage::JournalEvent::GateOpenedBoundEvent {
            run: gate.gate.run,
            seq,
            gate: gate.clone(),
            opened_at_ms,
            requester: requester.clone(),
        };
        let encoded =
            postcard::to_allocvec(&event).map_err(|_| crate::RuntimeError::EncodeFailed)?;
        u64::try_from(encoded.len()).map_err(|_| crate::RuntimeError::EncodeFailed)
    }

    fn gate_decision_bound_event_bytes(
        record: &vb_core::manual_gate::BoundGateOutcomeRecord,
        seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<u64> {
        let event = vb_storage::JournalEvent::GateDecisionBoundEvent {
            run: record.record.run,
            seq,
            record: Box::new(record.clone()),
        };
        let encoded =
            postcard::to_allocvec(&event).map_err(|_| crate::RuntimeError::EncodeFailed)?;
        u64::try_from(encoded.len()).map_err(|_| crate::RuntimeError::EncodeFailed)
    }
}
