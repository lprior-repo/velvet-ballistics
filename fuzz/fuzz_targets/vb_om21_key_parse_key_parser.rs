#![no_main]
// Obligation: PO-vb-om21-key-parse-fuzz
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if data.len() == 17 && data[0] == 0x11 {
        let _seq = u64::from_be_bytes([
            data[9], data[10], data[11], data[12], data[13], data[14], data[15], data[16],
        ]);
    }
});
