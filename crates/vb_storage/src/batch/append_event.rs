#![forbid(unsafe_code)]
use crate::codec::encode_record;
use crate::constants::{
    MAGIC_JOURNAL_EVENT, MAX_BATCH_COUNT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
};
use crate::error::JournalError;
use crate::events::JournalEvent;
use crate::keys::run_event_key;
use super::types::JournalWriteBatch;

impl<'j> JournalWriteBatch<'j> {
    /// Appends a journal event into the batch.
    ///
    /// # Invariant I20
    /// Duplicate event detection is enforced at `append_event` time by
    /// checking the journal's keyspace for already-committed events.
    /// Same-batch idempotent inserts are allowed (duplicates within
    /// the same batch are collapsed at commit time).
    ///
    /// # Guard Precedence (C6)
    /// 1. Key construction
    /// 2. Same-batch duplicate check (HashSet guard)
    /// 3. Durable duplicate check → aborts batch
    /// 4. Count capacity check (QueueFull)
    /// 5. Per-record encoding / payload size check (PayloadTooLarge)
    /// 6. Accumulated byte admission check (JournalBatchBytesExceeded)
    /// 7. Insert into inner OwnedWriteBatch
    ///
    /// # Preconditions (requires)
    /// - The batch is not already aborted.
    /// - `event.run_id()` and `event.seq()` form a valid key.
    /// - `event` payload is bounded by `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`.
    ///
    /// # Postconditions (ensures)
    /// - On success: the event is staged in `inner`, `staged_bytes` is
    ///   incremented by the full encoded record length.
    /// - On `DuplicateEvent`: batch is aborted, no state mutated.
    /// - On `QueueFull`: no state mutated, batch remains open.
    /// - On `PayloadTooLarge`: no state mutated.
    /// - On `JournalBatchBytesExceeded`: no state mutated,
    ///   `staged_bytes` unchanged, batch remains open.
    pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError> {
        let key = run_event_key(event.run_id(), event.seq())?;
        // Same-batch duplicate guard (vb-1rqz7.18 / vb-byk3q / SA-003).
        //
        // `journal.events.contains_key` only inspects the durable
        // memtable; it cannot see entries staged into the current
        // `OwnedWriteBatch` but not yet committed. Without this
        // HashSet-based guard, two `append_event` calls with the same
        // `(run, seq)` would both pass the durable check and Fjall
        // would silently overwrite the first value at commit time.
        if self.staged_event_keys.contains(&key) {
            return Err(JournalError::DuplicateStagedKey {
                run: event.run_id(),
                seq: event.seq(),
            });
        }
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

        // Byte admission check: guard 6 per C6 contract.
        //
        // Uses checked_add to avoid overflow; overflow is rejected
        // with the same JournalBatchBytesExceeded error as a budget
        // overrun.  The encoded_len conversion is try_from on
        // principle, though the bounded payload guarantees it always
        // fits in u64 on all practical targets.
        if let Some(limit) = self.byte_limit {
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

        self.inner.insert(&self.journal.events, key, value);
        // Record the key as staged so a subsequent append_event with
        // the same `(run, seq)` is rejected by the same-batch
        // duplicate guard above before the durable lookup.
        self.staged_event_keys.insert(key);
        Ok(())
    }
}