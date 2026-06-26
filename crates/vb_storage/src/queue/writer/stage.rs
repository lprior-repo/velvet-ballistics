use crate::{
    codec::{decode_journal_event, encode_record},
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    events::JournalEvent,
    journal::FjallJournal,
    keys::run_event_key,
};

/// Stages one queued event into the supplied `OwnedWriteBatch`.
///
/// Idempotency: when the durable events keyspace already holds a value
/// at the same `(run, seq)`, the existing bytes are decoded and compared
/// against the queued event. A match means an idempotent retry — the
/// event is silently skipped so the queue's eventual drain remains
/// correct. A mismatch returns `DuplicateEvent` so the operator can
/// diagnose the divergence.
pub(super) fn stage_queued_event(
    owned_batch: &mut fjall::OwnedWriteBatch,
    journal: &FjallJournal,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    let key = run_event_key(event.run_id(), event.seq())?;
    if let Some(existing_bytes) = journal.events.get(key.as_slice())? {
        let (_, existing) = decode_journal_event(
            existing_bytes.as_ref(),
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        if existing == *event {
            return Ok(());
        }
        return Err(JournalError::DuplicateEvent {
            run: event.run_id(),
            seq: event.seq(),
        });
    }
    let value = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    owned_batch.insert(&journal.events, key, value);
    Ok(())
}
