//! Fuzz target for journal event decode roundtrip.
//!
//! This target verifies that `decode_record::<JournalEvent>` never panics on any input and
//! that successful decodes always produce valid events (B11, B12 from LETHAL-7).
//!
//! Corpus seeds are maintained in `fuzz/corpus/journal_event/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Journal event deserialization must not panic for any input.
    // This is B11: For any `data: &[u8]`, decode must not panic.
    let result = vb_storage::codec::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    // B12: If decode succeeds, the event must pass structural validation.
    if let Ok((_envelope, event)) = result {
        assert!(
            event.is_valid(),
            "decode succeeded but is_valid() returned false"
        );
    }
});
