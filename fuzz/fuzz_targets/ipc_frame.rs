//! Fuzz target for IPC frame header/payload decoding.
//!
//! This target verifies that `decode_frame_header` and `decode_frame_payload`
//! never panic on arbitrary input and return typed Result values for both
//! valid and corrupt frames.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_ipc_frame(data);
});
