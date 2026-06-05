//! Fuzz target: ipc_decode.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_ipc_decode(data);
});
