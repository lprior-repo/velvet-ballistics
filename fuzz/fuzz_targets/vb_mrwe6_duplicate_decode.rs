#![no_main]

//! Fuzz target for obl-vb-in8ib-duplicate-fuzz.
//!
//! Exercises arbitrary persisted journal bytes through production
//! `decode_record::<JournalEvent>` and routes decoded duplicate facts into the
//! MRWE6 production seam decision. Malformed bytes must remain typed decode
//! errors; divergent decoded events must not classify as idempotent success.

use libfuzzer_sys::fuzz_target;
use vb_storage::mrwe6_seams::{
    Mrwe6DuplicateRetryDecision, mrwe6_duplicate_retry_decision,
};

fuzz_target!(|data: &[u8]| {
    let Some((&selector, payload)) = data.split_first() else {
        return;
    };

    let existing = decode_journal_event(payload);
    let retry = decode_journal_event(data);

    if let (Ok(existing_event), Ok(decoded_retry_event)) = (existing, retry) {
        let retry_event = if selector & 1 == 0 {
            existing_event.clone()
        } else {
            decoded_retry_event
        };
        let marker_present = selector & 2 != 0;
        let decision = mrwe6_duplicate_retry_decision(&existing_event, &retry_event, marker_present);
        if existing_event != retry_event {
            assert_eq!(
                decision,
                Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict
            );
        }
    }
});

fn decode_journal_event(data: &[u8]) -> Result<vb_storage::JournalEvent, vb_storage::JournalError> {
    vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map(|(_, event)| event)
}
