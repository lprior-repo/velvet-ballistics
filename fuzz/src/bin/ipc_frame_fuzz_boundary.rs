//! Fuzz target: ipc_frame_fuzz_boundary.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_ipc_frame_boundary(data);
});
