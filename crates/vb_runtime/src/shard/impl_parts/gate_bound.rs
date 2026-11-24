// Strict durable ports for the additive bound gate events (RecordKind 54/55).

impl Shard {
    /// Strict durable bound gate-opening port. Mirrors the legacy port with
    /// the additive `BoundManualGate` payload and event kind.
    pub(crate) fn append_gate_opened_bound_strict(
        &mut self,
        gate: vb_core::manual_gate::BoundManualGate,
        opened_at_ms: u64,
        requester: vb_core::manual_gate::ApproverId,
    ) -> crate::RuntimeResult<vb_storage::EventSeq> {
        let run = gate.gate.run;
        if !gate.is_structurally_valid() || !requester.is_valid() {
            return Err(crate::RuntimeError::InvalidGateOpenedRecord);
        }
        self.append_gate_opened_bound_record(gate, opened_at_ms, requester, run)
    }

    fn append_gate_opened_bound_record(
        &mut self,
        gate: vb_core::manual_gate::BoundManualGate,
        opened_at_ms: u64,
        requester: vb_core::manual_gate::ApproverId,
        run: vb_core::ids::RunId,
    ) -> crate::RuntimeResult<vb_storage::EventSeq> {
        let seq = self.journal_sequence_for(run);
        let next = Self::checked_next_seq(seq)?;
        self.reserve_journal_sequence_slot(run)?;
        let event_bytes =
            Self::gate_opened_bound_event_bytes(&gate, opened_at_ms, &requester, seq)?;
        let projected = self.projected_bound_gate_bytes(run, gate.gate.id, event_bytes)?;
        if self.bound_gate_journal_exceeds_budget(run, projected)? {
            return Err(crate::RuntimeError::GateJournalBudgetExceeded { gate: gate.gate.id });
        }
        self.journal
            .append_gate_opened_bound_strict(gate, opened_at_ms, requester, seq)?;
        let _previous = self.journal_sequences.insert(run, next);
        let state = self
            .run_state_get_mut(run)
            .ok_or(crate::RuntimeError::RunNotFound)?;
        state.journal_bytes_written = projected;
        Ok(seq)
    }

    fn projected_bound_gate_bytes(
        &self,
        run: vb_core::ids::RunId,
        gate: vb_core::manual_gate::GateId,
        event_bytes: u64,
    ) -> crate::RuntimeResult<u64> {
        self.run_state_get(run)
            .map(|state| state.journal_bytes_written)
            .ok_or(crate::RuntimeError::RunNotFound)?
            .checked_add(event_bytes)
            .ok_or(crate::RuntimeError::GateJournalBudgetExceeded { gate })
    }

    fn bound_gate_journal_exceeds_budget(
        &self,
        run: vb_core::ids::RunId,
        projected: u64,
    ) -> crate::RuntimeResult<bool> {
        let state = self.run_state_get(run).ok_or(crate::RuntimeError::RunNotFound)?;
        Ok(Self::journal_bytes_exceeds_budget(state, projected))
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
        let projected = self.projected_bound_gate_bytes(run, record.record.gate, event_bytes)?;
        if self.bound_gate_journal_exceeds_budget(run, projected)? {
            return Err(crate::RuntimeError::GateJournalBudgetExceeded {
                gate: record.record.gate,
            });
        }
        self.journal
            .append_gate_decision_bound_strict(record, seq)?;
        let _previous = self.journal_sequences.insert(run, next);
        let state = self
            .run_state_get_mut(run)
            .ok_or(crate::RuntimeError::RunNotFound)?;
        state.journal_bytes_written = projected;
        Ok(())
    }

    /// Strict atomic port: journals a bound gate denial or timeout and the
    /// `RunFailed` it causes as one batch, so the gate cannot close without the
    /// run failure being durable. Returns the projected run journal bytes, or
    /// `None` with nothing written when that batch exceeds the run's budget.
    pub(crate) fn append_bound_gate_failure_strict(
        &mut self,
        record: vb_core::manual_gate::BoundGateOutcomeRecord,
    ) -> crate::RuntimeResult<Option<u64>> {
        let run = record.record.run;
        if !record.is_structurally_valid() {
            return Err(crate::RuntimeError::InvalidGateDecisionRecord);
        }
        let seq = self.journal_sequence_for(run);
        let run_failed_seq = Self::checked_next_seq(seq)?;
        let next = Self::checked_next_seq(run_failed_seq)?;
        self.reserve_journal_sequence_slot(run)?;
        let run_failed = self.bound_gate_failure_run_failed(run)?;
        let Some(projected) =
            self.project_bound_gate_failure_bytes(&record, seq, run_failed_seq, &run_failed)?
        else {
            return Ok(None);
        };
        self.journal
            .append_gate_decision_bound_failing_run_strict(record, seq, run_failed)?;
        let _previous = self.journal_sequences.insert(run, next);
        Ok(Some(projected))
    }

    fn bound_gate_failure_run_failed(
        &self,
        run: vb_core::ids::RunId,
    ) -> crate::RuntimeResult<crate::journal::RuntimeJournalEvent> {
        let state = self
            .run_state_get(run)
            .ok_or(crate::RuntimeError::RunNotFound)?;
        Ok(crate::journal::RuntimeJournalEvent::RunFailed {
            run,
            attempt: state.run_attempt.get(),
        })
    }

    /// Projected run journal bytes for a bound gate failure batch, or `None`
    /// when that batch exceeds the run's journal budget.
    fn project_bound_gate_failure_bytes(
        &self,
        record: &vb_core::manual_gate::BoundGateOutcomeRecord,
        seq: vb_storage::EventSeq,
        run_failed_seq: vb_storage::EventSeq,
        run_failed: &crate::journal::RuntimeJournalEvent,
    ) -> crate::RuntimeResult<Option<u64>> {
        let state = self
            .run_state_get(record.record.run)
            .ok_or(crate::RuntimeError::RunNotFound)?;
        let decision_bytes = Self::gate_decision_bound_event_bytes(record, seq)?;
        let failed_bytes = crate::journal::charged_batch_bytes(
            self.journal.as_ref(),
            core::slice::from_ref(run_failed),
            run_failed_seq,
        )?;
        let projected = state
            .journal_bytes_written
            .checked_add(decision_bytes)
            .and_then(|bytes| bytes.checked_add(failed_bytes))
            .ok_or(crate::RuntimeError::GateJournalBudgetExceeded {
                gate: record.record.gate,
            })?;
        Ok((!Self::journal_bytes_exceeds_budget(state, projected)).then_some(projected))
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
