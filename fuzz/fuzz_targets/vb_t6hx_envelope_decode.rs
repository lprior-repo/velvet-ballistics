#![no_main]
use libfuzzer_sys::fuzz_target;
use vb_storage::codec::decode_journal_event;
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES};
use vb_storage::error::JournalError;

fuzz_target!(|data: &[u8]| {
    let result = decode_journal_event(data, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    if data.len() < RECORD_HEADER_BYTES {
        assert!(matches!(result, Err(JournalError::UnexpectedEof)));
    }
    if matches!(result, Err(JournalError::PostcardDecodeFailed)) {
        assert!(data.len() >= RECORD_HEADER_BYTES);
    }
});
