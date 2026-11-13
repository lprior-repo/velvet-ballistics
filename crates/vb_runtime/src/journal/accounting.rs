/// Exact charged wire size of one durable storage event.
pub(crate) fn storage_event_bytes(event: &JournalEvent) -> RuntimeResult<u64> {
    let encoded = postcard::to_allocvec(event).map_err(|_| RuntimeError::EncodeFailed)?;
    u64::try_from(encoded.len()).map_err(|_| RuntimeError::EncodeFailed)
}

/// Exact bytes charged for a runtime batch.
///
/// Durable journals charge the actual storage wire encoding, including the
/// per-run sequence field, which is what recovery later sums from the durable
/// event stream. Volatile/noop journals have no storage encoding and keep the
/// in-memory runtime encoding.
pub(crate) fn charged_batch_bytes(
    journal: &dyn RuntimeJournal,
    events: &[RuntimeJournalEvent],
    start_seq: EventSeq,
) -> RuntimeResult<u64> {
    let storage_backed = journal.storage_journal().is_some();
    let mut seq = start_seq;
    let mut total = 0_u64;
    for event in events {
        let bytes = if storage_backed {
            let storage = journal.convert_to_storage_event(event.clone(), seq)?;
            storage_event_bytes(&storage)?
        } else {
            let encoded = postcard::to_allocvec(event).map_err(|_| RuntimeError::EncodeFailed)?;
            u64::try_from(encoded.len()).map_err(|_| RuntimeError::EncodeFailed)?
        };
        total = total.checked_add(bytes).ok_or(RuntimeError::EncodeFailed)?;
        seq = EventSeq::new(
            seq.get()
                .checked_add(1)
                .ok_or(vb_storage::JournalError::SequenceOverflow)?,
        );
    }
    Ok(total)
}
