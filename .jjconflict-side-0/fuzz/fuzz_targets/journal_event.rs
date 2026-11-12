//! Fuzz target for journal event decode roundtrip.
//!
//! Uses the shared oracle in `fuzz_lib::journal_target::event` so the libFuzzer
//! target, stdin smoke wrappers, and regression tests exercise the same typed
//! decode/error and encode/decode round-trip checks.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_journal_event(data);
});
