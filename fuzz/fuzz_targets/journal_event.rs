//! Fuzz target for journal event decode roundtrip.
//!
//! Verifies that `decode_journal_event` never panics and returns typed errors
//! (not panics) for all inputs, per B11. Also verifies that when a decode succeeds,
//! the resulting event is always semantically valid (B12) — the validate-or-error
//! gate in `decode_journal_event` ensures this invariant holds.
//!
//! Corpus seeds are maintained in `fuzz/corpus/journal_event/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Guard against empty input — decode_journal_event returns UnexpectedEof but
    // we return early for defense-in-depth clarity.
    if data.is_empty() {
        return;
    }

    // B11: For any `data: &[u8]`, decode must not panic and must return a typed error.
    // B12: If decode succeeds, the event is always semantically valid (is_valid() = true)
    //      because decode_journal_event validates this at decode time.
    let result = vb_storage::codec::decode_journal_event(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    // The assertion is now defense-in-depth: decode_journal_event already guarantees
    // is_valid() == true for all Ok results, so this branch should always pass.
    if let Ok((_envelope, event)) = result {
        debug_assert!(
            event.is_valid(),
            "decode_journal_event returned Ok but event.is_valid() = false"
        );
    }
});
