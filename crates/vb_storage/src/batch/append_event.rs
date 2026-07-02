#![forbid(unsafe_code)]
use super::types::JournalWriteBatch;
use crate::codec::encode_record;
use crate::constants::{MAGIC_JOURNAL_EVENT, MAX_BATCH_COUNT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use crate::error::JournalError;
use crate::events::JournalEvent;
use crate::keys::run_event_key;
use crate::types::EventSeq;
use vb_core::RunId;

impl<'j> JournalWriteBatch<'j> {
    /// Appends a journal event into the batch.
    ///
    /// # Invariant I20
    /// Duplicate event detection is enforced at `append_event` time by
    /// checking both the current batch's staged keys and the journal's
    /// keyspace for already-committed events. Same-batch duplicate
    /// keys are rejected before Fjall can collapse them silently.
    ///
    /// # Guard Precedence (C6)
    /// 1. Key construction
    /// 2. Semantic event validation
    /// 3. **Next-sequence-at-write guard** (vb-r8oso NEW) — verifies
    ///    `event.seq() == next_sequence_at_write(event.run_id())`
    ///    before the same-batch or durable duplicate check. A
    ///    mismatch is rejected with `SequenceMismatch { run, expected,
    ///    actual }` and the batch is aborted so subsequent
    ///    `append_event` calls surface `BatchAborted`.
    /// 4. Same-batch duplicate check (HashSet guard)
    /// 5. Durable duplicate check → aborts batch
    /// 6. Count capacity check (QueueFull)
    /// 7. Per-record encoding / payload size check (PayloadTooLarge)
    /// 8. Accumulated byte admission check (JournalBatchBytesExceeded)
    /// 9. Insert into inner OwnedWriteBatch
    ///
    /// # Preconditions (requires)
    /// - The batch is not already aborted.
    /// - `event.run_id()` and `event.seq()` form a valid key.
    /// - `event` payload is bounded by `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`.
    /// - `event.seq() == next_sequence_at_write(event.run_id())` at the
    ///   moment this function runs.
    ///
    /// # Postconditions (ensures)
    /// - On success: the event is staged in `inner`, `staged_bytes` is
    ///   incremented by the full encoded record length.
    /// - On `SequenceMismatch`: batch is aborted (`aborted = true`),
    ///   no state mutated. Callers must re-build the batch with the
    ///   correct expected seq.
    /// - On `DuplicateStagedKey`: no state mutated, batch remains open.
    /// - On `DuplicateEvent`: batch is aborted, no state mutated.
    /// - On `QueueFull`: no state mutated, batch remains open.
    /// - On `PayloadTooLarge`: no state mutated.
    /// - On `JournalBatchBytesExceeded`: no state mutated,
    ///   `staged_bytes` unchanged, batch remains open.
    pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError> {
        let key = run_event_key(event.run_id(), event.seq())?;
        if !event.is_valid() {
            return Err(JournalError::InvalidEvent);
        }
        // vb-r8oso: next-sequence-at-write guard. The expected seq
        // combines the durable keyspace answer (via
        // `FjallJournal::next_sequence_at_write`) with the
        // `staged_event_keys` accumulated earlier in the *same*
        // batch. A multi-event batch must see its previously-staged
        // entries reflected in the expected seq; otherwise the
        // second event in a fresh batch would always mismatch
        // (durable keyspace still empty, staged keyspace has seq=0).
        // A mismatch aborts the batch so `append_strict_batch` and
        // the strict append path reject the entire batch atomically
        // (no partial durable commit). The event's `seq` is NEVER
        // rewritten.
        let expected = self.next_expected_seq_for(event.run_id())?;
        if event.seq() != expected {
            self.aborted = true;
            return Err(JournalError::SequenceMismatch {
                run: event.run_id(),
                expected,
                actual: event.seq(),
            });
        }
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
        // vb-3wn7x: maintain the pending action index atomically with
        // the event write. The action lifecycle map (see
        // `super::action_index`) translates each event variant into the
        // index mutation it implies (insert for scheduled events,
        // tombstone for completed/failed/abandoned events, no-op for
        // every other variant). The mutation lands in the SAME
        // OwnedWriteBatch, so committing this batch makes the event
        // and the index update durable together — recovery can rely on
        // the index as the authoritative pending-action cursor.
        self.journal
            .stage_pending_action_index_op(&mut self.inner, event)?;
        // Record the key as staged so a subsequent append_event with
        // the same `(run, seq)` is rejected by the same-batch
        // duplicate guard above before the durable lookup.
        self.staged_event_keys.insert(key);
        Ok(())
    }

    /// Returns the next expected `EventSeq` for `run` considering both
    /// the durable keyspace and the events already staged in this
    /// batch. Backs the [`Self::append_event`] next-sequence-at-write
    /// guard (vb-r8oso).
    ///
    /// Implementation: starts with
    /// `self.journal.next_sequence_at_write(run)` (durable keyspace
    /// answer), then walks `staged_event_keys` for any keys with a
    /// matching run prefix and raises the floor to
    /// `max_staged_seq_for_run + 1`. The walk is bounded by
    /// `MAX_BATCH_COUNT` (a static bound), so the loop has a fixed
    /// upper bound.
    fn next_expected_seq_for(&self, run: RunId) -> Result<EventSeq, JournalError> {
        let mut expected = self.journal.next_sequence_at_write(run)?;
        // Walk staged keys (bounded by MAX_BATCH_COUNT) and raise the
        // expected seq if any prior staged event for the same run has
        // a higher seq. The loop is a static upper bound by
        // construction (HashSet has at most MAX_BATCH_COUNT entries).
        for staged_key in &self.staged_event_keys {
            // Skip keys for a different run (prefix + run_id).
            if staged_key.len() != crate::constants::JOURNAL_KEY_BYTES {
                continue;
            }
            if staged_key[0] != crate::constants::PREFIX_RUN_EVENT {
                continue;
            }
            let staged_run_bytes: [u8; 8] = staged_key
                .get(1..9)
                .and_then(|s| s.try_into().ok())
                .ok_or(JournalError::MalformedKeyspaceRow {
                    prefix: crate::constants::PREFIX_RUN_EVENT,
                    expected_len: crate::constants::JOURNAL_KEY_BYTES,
                    actual_len: staged_key.len(),
                })?;
            let staged_run = RunId::new(u64::from_be_bytes(staged_run_bytes));
            if staged_run != run {
                continue;
            }
            let staged_seq_bytes: [u8; 8] = staged_key
                .get(9..17)
                .and_then(|s| s.try_into().ok())
                .ok_or(JournalError::MalformedKeyspaceRow {
                    prefix: crate::constants::PREFIX_RUN_EVENT,
                    expected_len: crate::constants::JOURNAL_KEY_BYTES,
                    actual_len: staged_key.len(),
                })?;
            let staged_seq = u64::from_be_bytes(staged_seq_bytes);
            let candidate = match crate::codec::next_seq(EventSeq::new(staged_seq)) {
                Ok(s) => s,
                Err(JournalError::SequenceOverflow) => {
                    // The staged seq is already at MAX; the only legal
                    // next seq is the overflow error itself, which
                    // surfaces to the caller as `SequenceMismatch` with
                    // `expected == EventSeq::MAX` (which never matches
                    // any caller-supplied `event.seq()`, since the
                    // codec reserves MAX as the saturation sentinel).
                    return Err(JournalError::SequenceOverflow);
                }
                Err(other) => return Err(other),
            };
            if candidate.get() > expected.get() {
                expected = candidate;
            }
        }
        Ok(expected)
    }
}
