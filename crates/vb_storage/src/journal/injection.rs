use crate::{
    codec::encode_record,
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    journal::FjallJournal,
    keys::run_event_key,
    types::EventSeq,
};

impl FjallJournal {
    /// Injects a raw event into the journal.
    ///
    /// DANGER: Bypassess all runtime event sequencing and admission rules.
    /// Used primarily for disaster recovery and test setup.
    pub fn inject_raw_event(
        &self,
        run: vb_core::RunId,
        seq: EventSeq,
        kind: crate::records::RecordKind,
        payload: &[u8],
    ) -> Result<(), JournalError> {
        let key = run_event_key(run, seq)?;
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

    /// Injects a sequence gap marker to allow replaying past a known gap.
    ///
    /// DANGER: This is an expert recovery tool.
    pub fn inject_seq_gap(
        &self,
        run: vb_core::RunId,
        gap_seq: EventSeq,
    ) -> Result<(), JournalError> {
        let key = run_event_key(run, gap_seq)?;
        // Injected gaps use an empty record that specifically doesn't match normal
        // event serialization, but we can encode it as a placeholder.
        let value = encode_record(
            MAGIC_JOURNAL_EVENT,
            crate::records::RecordKind::RunCancelled, // Valid kind for journal events
            gap_seq.get(),
            &(), // Empty payload - decode will succeed but be meaningless
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        self.events.insert(key.to_vec(), value)?;
        Ok(())
    }
}
