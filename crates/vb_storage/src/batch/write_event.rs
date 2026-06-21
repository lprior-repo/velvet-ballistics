//! Journal-event staging for [`super::JournalWriteBatch`].

use super::{BatchState, JournalWriteBatch};
use crate::{
    codec::encode_record,
    constants::{MAGIC_JOURNAL_EVENT, MAX_BATCH_COUNT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    events::JournalEvent,
    keys::run_event_key,
};

impl<'j> JournalWriteBatch<'j> {
/// Appends a journal event into the batch.
///
/// Guard order: aborted-state check, key construction, in-flight
/// (staged) duplicate check, durable (committed) duplicate check,
/// count capacity, per-record encoding, accumulated byte admission,
/// then inner insert.
    pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError> {
        if self.state.is_aborted() {
            return Err(JournalError::DuplicateEvent {
                run: event.run_id(),
                seq: event.seq(),
            });
        }
        let key = run_event_key(event.run_id(), event.seq())?;
        if self.staged_event_keys.contains(&key) {
            self.state = BatchState::Aborted;
            return Err(JournalError::DuplicateEvent {
                run: event.run_id(),
                seq: event.seq(),
            });
        }
        if self.journal.events.contains_key(key)? {
            self.state = BatchState::Aborted;
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

        let limit = self.byte_limit.as_u64();
        if limit > 0 {
            let encoded_len =
                u64::try_from(value.len()).map_err(|_| JournalError::SequenceOverflow)?;
            let attempted = match self.staged_bytes.checked_add(encoded_len) {
                Some(total) => total,
                None => {
                    return Err(JournalError::JournalBatchBytesExceeded {
                        attempted: u64::MAX,
                        limit,
                    });
                }
            };
            if attempted > limit {
                return Err(JournalError::JournalBatchBytesExceeded { attempted, limit });
            }
            self.staged_bytes = attempted;
        }

        self.staged_event_keys.insert(key);
        self.inner.insert(&self.journal.events, key, value);
        Ok(())
    }
}
