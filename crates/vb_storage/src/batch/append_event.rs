#![forbid(unsafe_code)]
use crate::codec::encode_record;
use crate::constants::{MAGIC_JOURNAL_EVENT, MAX_BATCH_COUNT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use crate::error::JournalError;
use crate::events::JournalEvent;
use crate::keys::run_event_key;
use super::types::JournalWriteBatch;
impl<'j> JournalWriteBatch<'j> {
    pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError> {
        let key = run_event_key(event.run_id(), event.seq())?;
        if self.journal.events.contains_key(key)? {
            self.aborted = true;
            return Err(JournalError::DuplicateEvent {
                run: event.run_id(),
                seq: event.seq(),
            });
        }
        if self.inner.len() >= MAX_BATCH_COUNT {
            return Err(JournalError::QueueFull);
        }
        let value = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        if let Some(limit) = self.byte_limit {
            let encoded_len = u64::try_from(value.len()).map_err(|_| JournalError::SequenceOverflow)?;
            let attempted = match self.staged_bytes.checked_add(encoded_len) {
                Some(total) => total,
                None => {
                    return Err(JournalError::JournalBatchBytesExceeded { attempted: u64::MAX, limit });
                }
            };
            if attempted > limit {
                return Err(JournalError::JournalBatchBytesExceeded { attempted, limit });
            }
            self.staged_bytes = attempted;
        }
        self.inner.insert(&self.journal.events, key, value);
        Ok(())
    }
    #[must_use]
    pub fn staged_event_keys(&self) -> &std::collections::HashSet<[u8; crate::constants::JOURNAL_KEY_BYTES]> {
        &self.staged_event_keys
    }
}
