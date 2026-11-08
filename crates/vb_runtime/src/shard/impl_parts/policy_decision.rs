impl Shard {
    /// Strict durable policy decision port.
    ///
    /// Checks pending ticket ownership, computes checked next sequence,
    /// reserves map capacity, measures the stored decision event and
    /// charges it to the run's cumulative journal-bytes counter before
    /// appending, calls the journal's strict port, and advances the
    /// per-run sequence only after success. On any failure the pending
    /// state and next sequence are unchanged.
    pub(crate) fn append_policy_decision_strict(
        &mut self,
        decision: vb_core::policy::DecisionRecord,
    ) -> crate::RuntimeResult<()> {
        let run = decision.ticket.run;

        // Verify shard owns this pending ticket.
        self.require_pending_action_ownership(decision.ticket)?;

        // Get current sequence, compute checked next BEFORE append.
        let seq = self.journal_sequence_for(run);
        let next = Self::checked_next_seq(seq)?;

        // Reserve map capacity before append.
        self.reserve_journal_sequence_slot(run)?;

        // Measure the stored decision event and project the run's
        // cumulative journal-bytes counter BEFORE any durable write.
        let event_bytes = Self::policy_decision_event_bytes(&decision, seq)?;
        let projected = self
            .run_state_get(run)
            .map(|state| state.journal_bytes_written)
            .ok_or(crate::RuntimeError::RunNotFound)?
            .checked_add(event_bytes)
            .ok_or(crate::RuntimeError::EncodeFailed)?;

        // Call the journal strict port.
        self.journal.append_policy_decision_strict(decision, seq)?;

        // Publish next sequence only after successful append.
        let _previous = self.journal_sequences.insert(run, next);

        // Record the charged bytes in the run's durable counter.
        let state = self
            .run_state_get_mut(run)
            .ok_or(crate::RuntimeError::RunNotFound)?;
        state.journal_bytes_written = projected;

        Ok(())
    }

    /// Postcard bytes of the stored policy-decision event.
    ///
    /// The decision is stored as the whole storage `JournalEvent`, so the
    /// measure covers the stored record including its run and sequence.
    fn policy_decision_event_bytes(
        decision: &vb_core::policy::DecisionRecord,
        seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<u64> {
        let event = vb_storage::JournalEvent::PolicyDecisionEvent {
            run: decision.ticket.run,
            seq,
            decision: decision.clone(),
        };
        let encoded =
            postcard::to_allocvec(&event).map_err(|_| crate::RuntimeError::EncodeFailed)?;
        u64::try_from(encoded.len()).map_err(|_| crate::RuntimeError::EncodeFailed)
    }

    /// Compute the checked next sequence number for a run.
    ///
    /// Returns an error on u64 overflow. Must be called before any
    /// irreversible state mutation.
    fn checked_next_seq(seq: vb_storage::EventSeq) -> crate::RuntimeResult<vb_storage::EventSeq> {
        seq.get()
            .checked_add(1)
            .map(vb_storage::EventSeq::new)
            .ok_or_else(|| crate::RuntimeError::from(vb_storage::JournalError::SequenceOverflow))
    }
}
