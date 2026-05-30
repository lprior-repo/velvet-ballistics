//! Fuzz target for IPC frame decode.
//!
//! This target verifies that `decode_frame_header` and `decode_frame_payload` never
//! panic on any input bytes.
//!
//! Corpus seeds are maintained in `fuzz/corpus/ipc_frame/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_ipc_frame(data);
});
