#![no_main]

use fuzz_lib::boundary_inventory::parse_inventory;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _result = parse_inventory(data);
});
