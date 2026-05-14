//! IPC decode fuzz target
//!
//! Fuzz target for `decode_frame` from vb_ipc crate.
//! This target ensures the IPC frame decoder does not panic on malformed bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_ipc::frame::decode_frame;

fuzz_target!(|data: &[u8]| {
    // Decode must not panic - errors are Ok, panics are bugs
    let _ = decode_frame(data);
});
