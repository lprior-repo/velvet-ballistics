//! Fuzz target for `journal::parse_event`.
//!
//! This target verifies that `parse_event` never panics on any input and
//! that successful parses always produce valid events (B11, B12 from LETHAL-7).
//!
//! Corpus seeds are maintained in `fuzz/corpus/journal_event/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Journal event deserialization must not panic for any input.
    // This is B11: For any `data: &[u8]`, `parse_event(data)` must not panic.
    let result = vb_storage::journal::parse_event(data);

    // B12: If `parse_event(data)` returns `Ok(event)`, then `event.is_valid()` must be `true`.
    // This invariant holds because parse_event calls decode_record which validates the record
    // envelope, and is_valid() checks structural constraints that are preserved by postcard
    // serialization.
    if let Ok(event) = result {
        assert!(
            event.is_valid(),
            "parse_event succeeded but is_valid() returned false for {:?}",
            event
        );
    }
});
