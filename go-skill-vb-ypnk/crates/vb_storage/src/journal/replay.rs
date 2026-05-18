use crate::{
    codec::decode_record,
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    events::JournalEvent,
    journal::FjallJournal,
    keys::run_prefix_key,
    types::EventSeq,
};
use fjall::Readable;

impl FjallJournal {
    /// Replays one run's events in contiguous per-run sequence order.
    pub fn events_for_run(&self, run: vb_core::RunId) -> Result<Vec<JournalEvent>, JournalError> {
        let start_seq = self
            .latest_durable_snapshot_seq(run)
            .ok()
            .flatten()
            .unwrap_or(EventSeq::new(0));
        self.events_for_run_from(run, start_seq)
    }

    /// Returns events for a run starting from a given sequence, with validation.
    pub(crate) fn events_for_run_from(
        &self,
        run: vb_core::RunId,
        start_seq: EventSeq,
    ) -> Result<Vec<JournalEvent>, JournalError> {
        let mut replay = Vec::new();
        let mut expected = None;
        let snap = self.database.snapshot();

        for item in snap.prefix(&self.events, run_prefix_key(run)?) {
            let value = item.value()?;
            let (_, event): (_, JournalEvent) = decode_record(
                value.as_ref(),
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            )?;
            // Skip events before the start sequence (already captured in snapshot)
            if event.seq().get() < start_seq.get() {
                continue;
            }
            let expected_seq = expected.unwrap_or_else(|| event.seq());
            crate::codec::validate_replayed_event(run, expected_seq, &event)?;
            expected = Some(crate::codec::next_seq(expected_seq)?);
            replay.push(event);
        }

        Ok(replay)
    }
}
