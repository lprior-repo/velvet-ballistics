impl Shard {
    /// Strict durable policy decision port.
    ///
    /// Checks pending ticket ownership, computes checked next sequence,
    /// reserves map capacity, calls the journal's strict port, and advances
    /// the per-run sequence only after success. On any failure the pending
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

        // Call the journal strict port.
        self.journal.append_policy_decision_strict(decision, seq)?;

        // Publish next sequence only after successful append.
        let _previous = self.journal_sequences.insert(run, next);

        Ok(())
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
