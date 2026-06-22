use crate::{
    codec::encode_record,
    constants::{
        MAGIC_JOURNAL_EVENT, MAGIC_JOURNAL_SEQUENCE_GAP, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        MAX_RUN_SEQ_GAP_BYTES,
    },
    error::JournalError,
    journal::FjallJournal,
    keys::{run_event_key, run_seq_gap_key},
    records::RecordKind,
    types::EventSeq,
};

impl FjallJournal {
    /// Injects a raw event into the journal.
    ///
    /// Acquires the serialised write lock (so concurrent appenders cannot
    /// race) and rejects duplicate `(run, seq)` keys with
    /// [`JournalError::DuplicateEvent`] instead of silently overwriting an
    /// existing event. Used primarily for disaster recovery and test setup.
    pub fn inject_raw_event(
        &self,
        run: vb_core::RunId,
        seq: EventSeq,
        kind: RecordKind,
        payload: &[u8],
    ) -> Result<(), JournalError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        let key = run_event_key(run, seq)?;
        if self.events.contains_key(key)? {
            return Err(JournalError::DuplicateEvent { run, seq });
        }
        let value = encode_record(
            MAGIC_JOURNAL_EVENT,
            kind,
            seq.get(),
            &payload,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        self.events.insert(key.to_vec(), value)?;
        Ok(())
    }

    /// Records a disaster-recovery sequence-gap marker at `(run, gap_seq)`.
    ///
    /// The marker is written to the dedicated `run_seq_gap` keyspace with
    /// [`MAGIC_JOURNAL_SEQUENCE_GAP`] magic and [`RecordKind::SequenceGap`]
    /// wire ID 60. The marker is therefore never visible to
    /// `events_for_run`, `extract_terminal`, `apply_summary_event`, or any
    /// other journal-event consumer — it cannot be mis-decoded as
    /// `RunCancelled` (the prior bug) or any other lifecycle event.
    ///
    /// Acquires the serialised write lock and rejects both duplicate gap
    /// markers at the same `(run, gap_seq)` and existing real events at
    /// the same position with [`JournalError::DuplicateEvent`].
    pub fn inject_seq_gap(
        &self,
        run: vb_core::RunId,
        gap_seq: EventSeq,
    ) -> Result<(), JournalError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;

        let event_key = run_event_key(run, gap_seq)?;
        if self.events.contains_key(event_key)? {
            return Err(JournalError::DuplicateEvent { run, seq: gap_seq });
        }

        let gap_key = run_seq_gap_key(run, gap_seq)?;
        if self.run_seq_gap.contains_key(gap_key)? {
            return Err(JournalError::DuplicateEvent { run, seq: gap_seq });
        }

        let value = encode_record(
            MAGIC_JOURNAL_SEQUENCE_GAP,
            RecordKind::SequenceGap,
            gap_seq.get(),
            &(),
            MAX_RUN_SEQ_GAP_BYTES,
        )?;
        self.run_seq_gap.insert(gap_key.to_vec(), value)?;
        Ok(())
    }

    /// Returns `true` if a sequence-gap marker exists for `(run, seq)`.
    pub fn has_seq_gap_marker(
        &self,
        run: vb_core::RunId,
        seq: EventSeq,
    ) -> Result<bool, JournalError> {
        let key = run_seq_gap_key(run, seq)?;
        Ok(self.run_seq_gap.contains_key(key)?)
    }
}
